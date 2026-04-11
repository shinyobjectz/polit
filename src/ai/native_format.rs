use super::tools::ToolCall;
use super::memory::Exchange;
use super::DmMode;

/// Parsed response from Gemma 4 native format output
#[derive(Debug, Clone)]
pub struct ParsedResponse {
    /// Internal reasoning from <|channel>thought...<channel|>
    pub reasoning: Option<String>,
    /// Free text after stripping special tokens
    pub narration: String,
    /// Parsed tool calls from <|tool_call>...<tool_call|>
    pub tool_calls: Vec<ToolCall>,
}

/// Generate `<|tool>declaration:...<tool|>` blocks for available tools based on mode
pub fn tool_declarations(mode: DmMode) -> String {
    let tools = match mode {
        DmMode::CharacterCreation => creation_tool_defs(),
        _ => game_tool_defs(),
    };

    tools
        .iter()
        .map(|def| format!("<|tool>declaration:\n{}\n<tool|>", def))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Build full prompt in Gemma 4 native format
pub fn build_prompt(
    system: &str,
    tools: &str,
    history: &[Exchange],
    user_input: &str,
) -> String {
    let mut prompt = String::with_capacity(4096);

    // System turn with thinking enabled and tool declarations
    prompt.push_str("<bos><|turn>system\n");
    prompt.push_str(system);
    prompt.push_str("\n<|think|>\n");
    prompt.push_str(tools);
    prompt.push('\n');
    prompt.push_str("<turn|>\n");

    // Conversation history
    for ex in history {
        prompt.push_str("<|turn>user\n");
        prompt.push_str(&ex.user_input);
        prompt.push_str("<turn|>\n");
        prompt.push_str("<|turn>model\n");
        prompt.push_str(&ex.assistant_response);
        prompt.push_str("<turn|>\n");
    }

    // Current user input
    prompt.push_str("<|turn>user\n");
    prompt.push_str(user_input);
    prompt.push_str("<turn|>\n");
    prompt.push_str("<|turn>model\n");

    prompt
}

/// Build a continuation prompt with tool results injected
pub fn build_continuation_prompt(
    system: &str,
    tools: &str,
    history: &[Exchange],
    user_input: &str,
    tool_results: &[(String, String)],
) -> String {
    let mut prompt = build_prompt(system, tools, history, user_input);

    // Inject tool results before the model's next response
    // Remove the trailing "<|turn>model\n" so we can insert results first
    let model_turn = "<|turn>model\n";
    if prompt.ends_with(model_turn) {
        prompt.truncate(prompt.len() - model_turn.len());
    }

    // Add tool results
    for (tool_name, result) in tool_results {
        let tool_fn = tool_name.split(':').next().unwrap_or(tool_name).trim();
        prompt.push_str(&format!(
            "<|tool_result>response:{tool_fn}{{status:<|\"|>ok<|\"|>,detail:<|\"|>{result}<|\"|>}}<tool_result|>\n"
        ));
    }

    prompt.push_str("<|turn>model\n");
    prompt
}

/// Parse raw model output into structured response.
/// Extracts reasoning from `<|channel>...<channel|>`, tool calls from
/// `<|tool_call>...<tool_call|>`, and narration from remaining text.
pub fn parse_response(raw: &str) -> ParsedResponse {
    let mut reasoning = None;
    let mut tool_calls = Vec::new();
    let mut narration_parts = Vec::new();

    let mut remaining = raw;

    while !remaining.is_empty() {
        // Find the earliest special token
        let positions = [
            remaining.find("<|channel>").map(|p| (p, "channel")),
            remaining.find("<|tool_call>").map(|p| (p, "tool_call")),
            remaining.find("<|tool_result>").map(|p| (p, "tool_result")),
        ];

        let earliest = positions
            .into_iter()
            .flatten()
            .min_by_key(|(pos, _)| *pos);

        match earliest {
            Some((idx, "channel")) => {
                let before = remaining[..idx].trim();
                if !before.is_empty() {
                    narration_parts.push(before.to_string());
                }
                remaining = &remaining[idx + "<|channel>".len()..];
                if let Some(end) = remaining.find("<channel|>") {
                    let content = remaining[..end].trim();
                    let content = content.strip_prefix("thought").unwrap_or(content).trim();
                    if !content.is_empty() {
                        reasoning = Some(content.to_string());
                    }
                    remaining = &remaining[end + "<channel|>".len()..];
                } else {
                    let content = remaining.trim();
                    let content = content.strip_prefix("thought").unwrap_or(content).trim();
                    if !content.is_empty() {
                        reasoning = Some(content.to_string());
                    }
                    remaining = "";
                }
            }
            Some((idx, "tool_call")) => {
                let before = remaining[..idx].trim();
                if !before.is_empty() {
                    narration_parts.push(before.to_string());
                }
                remaining = &remaining[idx + "<|tool_call>".len()..];
                if let Some(end) = remaining.find("<tool_call|>") {
                    let call_str = remaining[..end].trim();
                    if let Some(tool) = parse_tool_call(call_str) {
                        tool_calls.push(tool);
                    }
                    remaining = &remaining[end + "<tool_call|>".len()..];
                } else {
                    let call_str = remaining.trim();
                    if let Some(tool) = parse_tool_call(call_str) {
                        tool_calls.push(tool);
                    }
                    remaining = "";
                }
            }
            Some((idx, "tool_result")) => {
                let before = remaining[..idx].trim();
                if !before.is_empty() {
                    narration_parts.push(before.to_string());
                }
                remaining = &remaining[idx + "<|tool_result>".len()..];
                if let Some(end) = remaining.find("<tool_result|>") {
                    remaining = &remaining[end + "<tool_result|>".len()..];
                } else {
                    remaining = "";
                }
            }
            _ => {
                // No more special tokens — rest is narration
                let text = strip_turn_tokens(remaining).trim().to_string();
                if !text.is_empty() {
                    narration_parts.push(text);
                }
                remaining = "";
            }
        }
    }

    let narration = narration_parts.join("\n\n");

    ParsedResponse {
        reasoning,
        narration,
        tool_calls,
    }
}

/// Parse a single tool call from native format.
/// Format: `call:func_name{key1:<|"|>val1<|"|>,key2:<|"|>val2<|"|>}`
fn parse_tool_call(raw: &str) -> Option<ToolCall> {
    let s = raw.strip_prefix("call:").unwrap_or(raw);

    // Find the function name (everything before the first '{')
    let brace_idx = s.find('{')?;
    let func_name = s[..brace_idx].trim();
    let args_str = &s[brace_idx + 1..];

    // Strip trailing '}'
    let args_str = args_str.strip_suffix('}').unwrap_or(args_str).trim();

    // Parse key-value pairs
    let args = parse_native_args(args_str);

    match func_name {
        "lock_field" => Some(ToolCall::LockField {
            field: args.get("field")?.clone(),
            value: args.get("value")?.clone(),
        }),
        "suggest_options" => {
            let options_str = args.get("options")?;
            let options: Vec<String> = options_str
                .split('|')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            Some(ToolCall::SuggestOptions {
                field: args.get("field")?.clone(),
                options,
                prompt: args.get("prompt")?.clone(),
            })
        }
        "ask_question" => Some(ToolCall::AskQuestion {
            topic: args.get("topic")?.clone(),
            question: args.get("question")?.clone(),
        }),
        "narrate" => Some(ToolCall::Narrate {
            text: args.get("text")?.clone(),
        }),
        "spawn_npc" => Some(ToolCall::SpawnNpc {
            name: args.get("name")?.clone(),
            role: args.get("role").cloned(),
            personality: args.get("personality").cloned(),
        }),
        "set_dc" => Some(ToolCall::SetDc {
            skill: args.get("skill")?.clone(),
            dc: args.get("dc")?.parse().ok()?,
            reason: args.get("reason")?.clone(),
        }),
        "trigger_event" => Some(ToolCall::TriggerEvent {
            event_type: args.get("event_type")?.clone(),
            description: args.get("description")?.clone(),
        }),
        "modify_rel" => Some(ToolCall::ModifyRel {
            npc: args.get("npc")?.clone(),
            field: args.get("field")?.clone(),
            delta: args.get("delta")?.parse().ok()?,
        }),
        "update_var" => Some(ToolCall::UpdateVar {
            name: args.get("name")?.clone(),
            value: args.get("value")?.clone(),
        }),
        "grant_card" => Some(ToolCall::GrantCard {
            card_id: args.get("card_id")?.clone(),
            reason: args.get("reason")?.clone(),
        }),
        "revoke_card" => Some(ToolCall::RevokeCard {
            card_id: args.get("card_id")?.clone(),
            reason: args.get("reason")?.clone(),
        }),
        "set_mood" => Some(ToolCall::SetMood {
            npc: args.get("npc")?.clone(),
            mood: args.get("mood")?.clone(),
        }),
        "roll_dice" => Some(ToolCall::RollDice {
            skill: args.get("skill")?.clone(),
            dc: args.get("dc")?.parse().ok()?,
            modifier: args.get("modifier")?.parse().ok()?,
        }),
        "schedule_event" => Some(ToolCall::ScheduleEvent {
            event_type: args.get("event_type")?.clone(),
            description: args.get("description")?.clone(),
            weeks_ahead: args.get("weeks_ahead")?.parse().ok()?,
        }),
        "score_adjust" => Some(ToolCall::ScoreAdjust {
            metric: args.get("metric")?.clone(),
            delta: args.get("delta")?.parse().ok()?,
        }),
        _ => {
            tracing::warn!("Unknown native tool call: {}", func_name);
            None
        }
    }
}

/// Parse native arg format: `key1:<|"|>val1<|"|>,key2:<|"|>val2<|"|>,key3:123`
fn parse_native_args(raw: &str) -> std::collections::HashMap<String, String> {
    let mut args = std::collections::HashMap::new();
    let mut remaining = raw;

    while !remaining.is_empty() {
        remaining = remaining.trim_start_matches(',').trim();
        if remaining.is_empty() {
            break;
        }

        // Find key (everything before ':')
        let colon_idx = match remaining.find(':') {
            Some(i) => i,
            None => break,
        };
        let key = remaining[..colon_idx].trim().to_string();
        remaining = &remaining[colon_idx + 1..];

        // Check if value is string-delimited with <|"|>
        if remaining.starts_with("<|\"|>") {
            remaining = &remaining["<|\"|>".len()..];
            // Find closing <|"|>
            if let Some(end) = remaining.find("<|\"|>") {
                let value = remaining[..end].to_string();
                args.insert(key, value);
                remaining = &remaining[end + "<|\"|>".len()..];
            } else {
                // No closing delimiter — take rest
                args.insert(key, remaining.to_string());
                remaining = "";
            }
        } else {
            // Bare value (number, boolean) — read until comma or end
            let end = remaining.find(',').unwrap_or(remaining.len());
            let value = remaining[..end].trim().to_string();
            args.insert(key, value);
            remaining = &remaining[end..];
        }
    }

    args
}

/// Strip turn boundary tokens from text
fn strip_turn_tokens(raw: &str) -> &str {
    let mut s = raw;
    // Strip common trailing/leading tokens
    for token in &["<turn|>", "<|turn>model", "<|turn>user", "<bos>", "<eos>"] {
        s = s.trim_start_matches(token).trim_end_matches(token);
    }
    s.trim()
}

// --- Tool definitions for prompt construction ---

fn creation_tool_defs() -> Vec<String> {
    vec![
        tool_def("lock_field", "Save a character detail the player revealed. Use this EVERY TIME the player tells you something about their character.", &[
            ("field", "string", "One of: background, motivation, archetype, starting_office, party, traits, family, rival, secret, tone"),
            ("value", "string", "The value to save. For tone: a short description like 'dark comedy' or 'earnest drama'"),
        ]),
    ]
}

fn game_tool_defs() -> Vec<String> {
    vec![
        tool_def("narrate", "Display narration text to the player", &[
            ("text", "string", "The text to display"),
        ]),
        tool_def("spawn_npc", "Spawn a new NPC entity", &[
            ("name", "string", "NPC name"),
            ("role", "string", "NPC role or title"),
            ("personality", "string", "Brief personality description"),
        ]),
        tool_def("set_dc", "Set difficulty class for upcoming roll", &[
            ("skill", "string", "Skill being tested"),
            ("dc", "number", "Difficulty class"),
            ("reason", "string", "Why this DC"),
        ]),
        tool_def("trigger_event", "Trigger a game event", &[
            ("event_type", "string", "Event category"),
            ("description", "string", "What happens"),
        ]),
        tool_def("modify_rel", "Modify a relationship edge", &[
            ("npc", "string", "NPC name"),
            ("field", "string", "Relationship dimension"),
            ("delta", "number", "Change amount"),
        ]),
        tool_def("update_var", "Update a simulation variable", &[
            ("name", "string", "Variable name"),
            ("value", "string", "New value"),
        ]),
        tool_def("grant_card", "Grant the player a card", &[
            ("card_id", "string", "Card identifier"),
            ("reason", "string", "Why granted"),
        ]),
        tool_def("revoke_card", "Revoke a card from the player", &[
            ("card_id", "string", "Card identifier"),
            ("reason", "string", "Why revoked"),
        ]),
        tool_def("set_mood", "Set NPC emotional state", &[
            ("npc", "string", "NPC name"),
            ("mood", "string", "Emotional state"),
        ]),
        tool_def("roll_dice", "Trigger a dice roll / skill check", &[
            ("skill", "string", "Skill being tested"),
            ("dc", "number", "Difficulty class"),
            ("modifier", "number", "Roll modifier"),
        ]),
        tool_def("schedule_event", "Schedule a future event", &[
            ("event_type", "string", "Event category"),
            ("description", "string", "What will happen"),
            ("weeks_ahead", "number", "Weeks until event fires"),
        ]),
        tool_def("score_adjust", "Adjust player score/metrics", &[
            ("metric", "string", "Score metric name"),
            ("delta", "number", "Change amount"),
        ]),
    ]
}

/// Build a JSON-schema tool definition string
fn tool_def(name: &str, description: &str, params: &[(&str, &str, &str)]) -> String {
    let mut properties = Vec::new();
    let mut required = Vec::new();

    for (pname, ptype, pdesc) in params {
        properties.push(format!(
            "    \"{}\": {{\"type\": \"{}\", \"description\": \"{}\"}}",
            pname, ptype, pdesc
        ));
        required.push(format!("\"{}\"", pname));
    }

    format!(
        "{{\n  \"name\": \"{}\",\n  \"description\": \"{}\",\n  \"parameters\": {{\n    \"type\": \"object\",\n    \"properties\": {{\n{}\n    }},\n    \"required\": [{}]\n  }}\n}}",
        name,
        description,
        properties.join(",\n"),
        required.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_call_string_args() {
        let raw = "call:lock_field{field:<|\"|>background<|\"|>,value:<|\"|>former prosecutor<|\"|>}";
        let tool = parse_tool_call(raw).unwrap();
        match tool {
            ToolCall::LockField { field, value } => {
                assert_eq!(field, "background");
                assert_eq!(value, "former prosecutor");
            }
            _ => panic!("Expected LockField"),
        }
    }

    #[test]
    fn test_parse_tool_call_numeric_args() {
        let raw = "call:set_dc{skill:<|\"|>Persuasion<|\"|>,dc:16,reason:<|\"|>Davis is hostile<|\"|>}";
        let tool = parse_tool_call(raw).unwrap();
        match tool {
            ToolCall::SetDc { skill, dc, reason } => {
                assert_eq!(skill, "Persuasion");
                assert_eq!(dc, 16);
                assert_eq!(reason, "Davis is hostile");
            }
            _ => panic!("Expected SetDc"),
        }
    }

    #[test]
    fn test_parse_tool_call_mixed_types() {
        let raw = "call:modify_rel{npc:<|\"|>Davis<|\"|>,field:<|\"|>trust<|\"|>,delta:-10}";
        let tool = parse_tool_call(raw).unwrap();
        match tool {
            ToolCall::ModifyRel { npc, field, delta } => {
                assert_eq!(npc, "Davis");
                assert_eq!(field, "trust");
                assert_eq!(delta, -10);
            }
            _ => panic!("Expected ModifyRel"),
        }
    }

    #[test]
    fn test_parse_response_full() {
        let raw = "<|channel>thought\nPlayer mentioned being a prosecutor. Should lock background.\n<channel|>\n<|tool_call>call:lock_field{field:<|\"|>background<|\"|>,value:<|\"|>former prosecutor<|\"|>}<tool_call|>\nA prosecutor who's seen the system from the inside! What case made you decide?";

        let parsed = parse_response(raw);
        assert!(parsed.reasoning.is_some());
        assert!(parsed.reasoning.unwrap().contains("prosecutor"));
        assert_eq!(parsed.tool_calls.len(), 1);
        assert!(parsed.narration.contains("prosecutor"));
        assert!(parsed.narration.contains("What case"));
    }

    #[test]
    fn test_parse_response_no_reasoning() {
        let raw = "<|tool_call>call:set_mood{npc:<|\"|>Davis<|\"|>,mood:<|\"|>angry<|\"|>}<tool_call|>\nDavis slams the table.";

        let parsed = parse_response(raw);
        assert!(parsed.reasoning.is_none());
        assert_eq!(parsed.tool_calls.len(), 1);
        assert!(parsed.narration.contains("Davis slams"));
    }

    #[test]
    fn test_parse_response_narration_only() {
        let raw = "The morning sun rises over the capitol building.";
        let parsed = parse_response(raw);
        assert!(parsed.reasoning.is_none());
        assert!(parsed.tool_calls.is_empty());
        assert!(parsed.narration.contains("morning sun"));
    }

    #[test]
    fn test_parse_response_multiple_tool_calls() {
        let raw = "<|tool_call>call:lock_field{field:<|\"|>party<|\"|>,value:<|\"|>Republican<|\"|>}<tool_call|>\n<|tool_call>call:lock_field{field:<|\"|>starting_office<|\"|>,value:<|\"|>Mayor<|\"|>}<tool_call|>\nA Republican mayor! What drives you?";

        let parsed = parse_response(raw);
        assert_eq!(parsed.tool_calls.len(), 2);
        assert!(parsed.narration.contains("Republican mayor"));
    }

    #[test]
    fn test_build_prompt_structure() {
        let system = "You are the AI for POLIT.";
        let tools = tool_declarations(DmMode::CharacterCreation);
        let history: Vec<Exchange> = vec![];
        let prompt = build_prompt(system, &tools, &history, "Hello");

        assert!(prompt.starts_with("<bos><|turn>system"));
        assert!(prompt.contains("<|think|>"));
        assert!(prompt.contains("<|tool>declaration:"));
        assert!(prompt.contains("<|turn>user\nHello<turn|>"));
        assert!(prompt.ends_with("<|turn>model\n"));
    }

    #[test]
    fn test_build_prompt_with_history() {
        let system = "System prompt.";
        let tools = tool_declarations(DmMode::DungeonMaster);
        let history = vec![Exchange {
            turn: 1,
            user_input: "hello".into(),
            assistant_response: "Welcome!".into(),
            tool_calls_summary: vec![],
            timestamp_week: 1,
        }];
        let prompt = build_prompt(system, &tools, &history, "what now");

        assert!(prompt.contains("<|turn>user\nhello<turn|>"));
        assert!(prompt.contains("<|turn>model\nWelcome!<turn|>"));
        assert!(prompt.contains("<|turn>user\nwhat now<turn|>"));
    }

    #[test]
    fn test_tool_declarations_creation() {
        let decls = tool_declarations(DmMode::CharacterCreation);
        assert!(decls.contains("lock_field"));
        // Only lock_field for character creation — no ask_question, suggest_options, narrate
        assert!(!decls.contains("spawn_npc"));
        assert!(!decls.contains("roll_dice"));
    }

    #[test]
    fn test_tool_declarations_game() {
        let decls = tool_declarations(DmMode::DungeonMaster);
        assert!(decls.contains("spawn_npc"));
        assert!(decls.contains("set_dc"));
        assert!(decls.contains("modify_rel"));
        assert!(decls.contains("roll_dice"));
        assert!(decls.contains("schedule_event"));
    }

    #[test]
    fn test_parse_schedule_event() {
        let raw = "call:schedule_event{event_type:<|\"|>crisis<|\"|>,description:<|\"|>Budget vote<|\"|>,weeks_ahead:3}";
        let tool = parse_tool_call(raw).unwrap();
        match tool {
            ToolCall::ScheduleEvent { event_type, description, weeks_ahead } => {
                assert_eq!(event_type, "crisis");
                assert_eq!(description, "Budget vote");
                assert_eq!(weeks_ahead, 3);
            }
            _ => panic!("Expected ScheduleEvent"),
        }
    }

    #[test]
    fn test_continuation_prompt_has_tool_results() {
        let system = "System.";
        let tools = tool_declarations(DmMode::CharacterCreation);
        let results = vec![
            ("lock_field: background = prosecutor".to_string(), "ok".to_string()),
        ];
        let prompt = build_continuation_prompt(system, &tools, &[], "test", &results);
        assert!(prompt.contains("<|tool_result>"));
        assert!(prompt.contains("<tool_result|>"));
    }
}
