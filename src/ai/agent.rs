use super::context::GameContext;
use super::debug_log::{AgentTurnLog, DebugLog, TurnTimer};
use super::memory::ConversationMemory;
use super::native_format;
use super::tools::ToolCall;
use super::vfs::VirtualFs;
use super::{AiProvider, DmMode};

/// Result of a full agent turn (may involve multiple internal iterations)
pub struct AgentResponse {
    /// Final narration text for the player
    pub narration: String,
    /// Game tool calls that were executed
    pub tool_calls: Vec<ToolCall>,
    /// Number of internal reasoning iterations used
    pub iterations: u32,
}

/// The agent orchestrator — manages multi-turn reasoning,
/// conversation memory, VFS, and tool execution.
pub struct Agent {
    pub memory: ConversationMemory,
    pub vfs: VirtualFs,
    mode: DmMode,
    max_iterations: u32,
}

impl Agent {
    pub fn new(mode: DmMode) -> Self {
        Self {
            memory: ConversationMemory::new(6),
            vfs: VirtualFs::new(),
            mode,
            max_iterations: 3,
        }
    }

    pub fn with_memory(mode: DmMode, memory: ConversationMemory) -> Self {
        Self {
            memory,
            vfs: VirtualFs::new(),
            mode,
            max_iterations: 3,
        }
    }

    /// Run a full agent turn. Builds prompt with conversation history,
    /// sends to model, parses structured output for tool calls.
    ///
    /// Multi-turn: if the model produces tool calls but no narration,
    /// feed tool results back and let it continue (up to max_iterations).
    pub fn run_turn<F>(
        &mut self,
        user_input: &str,
        context: &GameContext,
        provider: &mut dyn AiProvider,
        mut tool_executor: F,
        step_callback: Option<&dyn Fn(super::async_chat::AgentStep)>,
    ) -> AgentResponse
    where
        F: FnMut(&ToolCall) -> Option<String>,
    {
        let emit_step = |step: super::async_chat::AgentStep| {
            if let Some(cb) = step_callback {
                cb(step);
            }
        };
        let timer = TurnTimer::start();
        let mut all_narration = Vec::new();
        let mut all_tools = Vec::new();
        let mut iterations = 0u32;
        let mut tool_results = Vec::new();
        let mut raw_output_log = String::new();

        for iter in 0..self.max_iterations {
            iterations = iter + 1;

            emit_step(super::async_chat::AgentStep::Generating(iterations));

            // Build prompt — include tool results from previous iteration if any
            let prompt = if tool_results.is_empty() {
                self.build_full_prompt(user_input, context)
            } else {
                self.build_continuation_prompt(user_input, context, &tool_results)
            };

            tracing::info!(
                "Agent iteration {} — prompt {} chars (~{} tokens)",
                iterations,
                prompt.len(),
                prompt.len() / 4
            );

            // Generate
            let response = match provider.generate(&prompt, self.mode) {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::error!("Agent error on iteration {}: {}", iterations, e);
                    if all_narration.is_empty() {
                        all_narration
                            .push("The narrator pauses, collecting their thoughts...".into());
                    }
                    break;
                }
            };

            raw_output_log.push_str(&format!(
                "--- iteration {} ---\nnarration: {}\ntools: {:?}\n",
                iterations,
                &response.narration[..response.narration.len().min(200)],
                response.tool_calls.len()
            ));

            // Emit reasoning step if the native parser found one
            // (reasoning is extracted by the provider from <|channel> blocks)
            // We check the raw output log for it
            if let Some(reasoning) = extract_reasoning_from_response(&response) {
                emit_step(super::async_chat::AgentStep::Thinking(reasoning));
            }

            // Collect narration
            if !response.narration.is_empty() {
                all_narration.push(response.narration.clone());
            }

            // In character creation mode, if the model didn't produce lock_field calls
            // but the user clearly provided info, infer the tool calls
            let mut response_tools = response.tool_calls.clone();
            if self.mode == DmMode::CharacterCreation {
                let has_lock = response_tools
                    .iter()
                    .any(|t| matches!(t, ToolCall::LockField { .. }));
                if !has_lock && !user_input.trim().is_empty() {
                    let inferred = infer_character_fields(user_input, context);
                    for tool in &inferred {
                        tracing::info!("Inferred tool: {}", summarize_tool(tool));
                    }
                    response_tools.extend(inferred);
                }
            }

            // Execute tool calls, collect results for potential next iteration
            tool_results.clear();
            for tool in &response_tools {
                match tool {
                    ToolCall::Narrate { text } => {
                        all_narration.push(text.clone());
                    }
                    // ask_question is not a real tool — if model still uses it,
                    // extract the question text as narration
                    ToolCall::AskQuestion { question, .. } | ToolCall::SuggestOptions { prompt: question, .. } => {
                        if !question.is_empty() {
                            tracing::info!("Extracting narration from legacy tool: {}", &question[..question.len().min(80)]);
                            all_narration.push(question.clone());
                        }
                    }
                    _ => {
                        let summary = summarize_tool(tool);
                        tracing::info!("Tool: {}", summary);
                        emit_step(super::async_chat::AgentStep::ToolExecuted(summary.clone()));
                        let result = tool_executor(tool);
                        tool_results.push((
                            summary,
                            result.unwrap_or_else(|| "ok".into()),
                        ));
                        all_tools.push(tool.clone());
                    }
                }
            }

            // Accept decision: do we have something to show the player?
            let has_narration = !all_narration.is_empty();
            let has_any_tools = !all_tools.is_empty();

            if has_narration {
                // Got narration — good enough, show it
                break;
            }
            if !has_narration && !has_any_tools {
                // Nothing at all — model produced empty output, break to avoid infinite loop
                break;
            }
            // Has tools but no narration — continue to next iteration
            // (the tool results will be fed back so model can produce narration)
        }

        // Store in memory
        let tool_summaries: Vec<String> = all_tools.iter().map(|t| summarize_tool(t)).collect();
        let final_narration = all_narration.join("\n\n");

        self.memory
            .add_exchange(user_input, &final_narration, &tool_summaries, context.week);

        // Debug log
        let mode_str = format!("{:?}", self.mode);
        DebugLog::log_turn(&AgentTurnLog {
            timestamp: chrono_now(),
            mode: mode_str,
            user_input: user_input.to_string(),
            prompt_chars: 0, // captured in iterations above
            prompt_est_tokens: 0,
            raw_output: raw_output_log,
            parsed_narration: final_narration.clone(),
            parsed_tool_calls: tool_summaries.clone(),
            duration_ms: timer.elapsed_ms(),
            iterations,
            memory_turns: self.memory.turn_count(),
            character_fields: vec![], // filled by caller if needed
        });

        tracing::info!(
            "Agent done: {} iterations, {} tools, {}ms, narration={} chars",
            iterations,
            all_tools.len(),
            timer.elapsed_ms(),
            final_narration.len()
        );

        AgentResponse {
            narration: final_narration,
            tool_calls: all_tools,
            iterations,
        }
    }

    /// Build the complete prompt with system instructions, game state,
    /// conversation history, and the current user input.
    /// Uses Gemma 4 native format with `<|turn>`, `<|tool>`, `<|think|>` tokens.
    fn build_full_prompt(&self, user_input: &str, context: &GameContext) -> String {
        let system = self.build_system_prompt(context);
        let game_ctx = self.build_game_context(context);

        let agent_notes = self.get_relevant_notes(context);
        let notes_block = if agent_notes.is_empty() {
            String::new()
        } else {
            format!("\n\nYOUR NOTES:\n{}", agent_notes)
        };

        let full_system = format!("{}\n\n{}{}", system, game_ctx, notes_block);
        let tools = native_format::tool_declarations(self.mode);
        let player_msg = format!("Player: {}", user_input);

        // Token budget: keep prompt under ~5000 chars (~1250 tokens)
        // to leave room for generation within 8192 context
        let max_prompt_chars = 5000;
        let history = self.memory.recent_exchanges();

        // Try with full history first, trim from oldest if too big
        let mut prompt = native_format::build_prompt(&full_system, &tools, history, &player_msg);
        if prompt.len() > max_prompt_chars && history.len() > 2 {
            // Trim oldest exchanges until it fits
            let mut trim_count = 1;
            while prompt.len() > max_prompt_chars && trim_count < history.len() - 1 {
                let trimmed = &history[trim_count..];
                prompt = native_format::build_prompt(&full_system, &tools, trimmed, &player_msg);
                trim_count += 1;
            }
            tracing::info!("Trimmed {} exchanges from history to fit token budget", trim_count);
        }

        prompt
    }

    /// Build a continuation prompt after tool execution (multi-turn)
    /// Uses Gemma 4 native `<|tool_result>` tokens.
    fn build_continuation_prompt(
        &self,
        user_input: &str,
        context: &GameContext,
        tool_results: &[(String, String)],
    ) -> String {
        let system = self.build_system_prompt(context);
        let game_ctx = self.build_game_context(context);
        let full_system = format!("{}\n\n{}", system, game_ctx);
        let tools = native_format::tool_declarations(self.mode);
        let player_msg = format!("Player: {}", user_input);

        native_format::build_continuation_prompt(
            &full_system,
            &tools,
            &self.memory.recent_exchanges(),
            &player_msg,
            tool_results,
        )
    }

    fn build_system_prompt(&self, context: &GameContext) -> String {
        let mode_instructions = match self.mode {
            DmMode::CharacterCreation => {
                "You are a sharp, funny creative partner. The player is telling you about a character \
                 for a political RPG. Your job is to GET INTO IT. Riff on everything they say. \
                 Be specific and engaged. Mirror the player's tone — if they're funny, be funny back. \
                 If they're serious, take them seriously. Never be generic.\n\n\
                 YES AND everything. Build on their ideas. Add vivid details they didn't think of. \
                 Make them laugh or make them think. Ask follow-up questions that pull on \
                 the most interesting thread.\n\n\
                 When they tell you something concrete about their character, quietly save it \
                 with lock_field in the background — but keep the conversation flowing naturally. \
                 The player should feel like they're talking to a friend, not filling out a form.\n\n\
                 TONE: Pay attention to how the player communicates. Are they funny, serious, \
                 dark, satirical, earnest, absurd? Once you have a clear sense of their vibe, \
                 lock_field 'tone' with a short description (e.g. 'dark comedy', 'earnest drama', \
                 'absurdist satire', 'gritty realism'). This sets the narrator's voice for the \
                 entire game.\n\n\
                 Valid fields for lock_field: background, motivation, archetype, starting_office, \
                 party, traits, family, rival, secret, tone"
            }
            DmMode::Narrator => {
                "You are the narrator. Generate morning briefings and describe consequences. \
                 Use narrate() for text. Use schedule_event() for upcoming developments. \
                 Keep responses to 3-5 sentences. Be vivid but concise."
            }
            DmMode::Conversation => {
                "You are voicing an NPC. Stay in character. Keep dialogue natural \
                 and brief (2-4 sentences). Use modify_rel() if the conversation shifts \
                 the relationship. Use set_mood() to reflect emotional state."
            }
            DmMode::DungeonMaster => {
                "You are the dungeon master. Respond to the player's actions with consequences \
                 and narrative. Use set_dc() for skill checks. Use trigger_event() for dramatic \
                 developments. Use grant_card() for milestone moments. 3-5 sentences."
            }
            DmMode::LawInterpreter => {
                "You are a legal analyst. Evaluate actions for legality. Be precise but accessible. \
                 Use narrate() for legal analysis."
            }
        };

        format!(
            "You are the AI for POLIT, an American politics simulator.\n\
             Tone: {}\n\n\
             {}\n\n\
             Use the declared tools via tool_call tokens. \
             Use channel thought to reason privately before responding.",
            context.tone_instructions, mode_instructions
        )
    }

    fn build_game_context(&self, context: &GameContext) -> String {
        let mut ctx = format!(
            "GAME STATE:\n\
             Week {}, Year {}\n\
             Player: {} ({})\n\
             AP: {}/{}",
            context.week,
            context.year,
            context.player_name,
            context.player_office,
            context.ap_current,
            context.ap_max,
        );

        if !context.economic_summary.is_empty() {
            ctx.push_str(&format!("\nEconomy: {}", context.economic_summary));
        }

        if !context.active_npcs.is_empty() {
            ctx.push_str("\n\nNPCs IN SCENE:");
            for npc in &context.active_npcs {
                ctx.push_str(&format!(
                    "\n  {} ({}) — mood: {}, trust: {}, respect: {}",
                    npc.name, npc.role, npc.mood, npc.trust, npc.respect
                ));
                for mem in &npc.recent_memories {
                    ctx.push_str(&format!("\n    memory: {}", mem));
                }
            }
        }

        if !context.recent_events.is_empty() {
            ctx.push_str("\n\nRECENT EVENTS:");
            for event in &context.recent_events {
                ctx.push_str(&format!("\n  - {}", event));
            }
        }

        ctx
    }

    fn get_relevant_notes(&self, context: &GameContext) -> String {
        let mut notes = Vec::new();

        if let Some(content) = self.vfs.read("notebook.md") {
            notes.push(format!("notebook.md:\n{}", truncate(content, 300)));
        }

        for npc in &context.active_npcs {
            let npc_file = format!("notes/{}.md", npc.name.to_lowercase().replace(" ", "_"));
            if let Some(content) = self.vfs.read(&npc_file) {
                notes.push(format!("{}:\n{}", npc_file, truncate(content, 200)));
            }
        }

        notes.join("\n\n")
    }

    pub fn set_mode(&mut self, mode: DmMode) {
        self.mode = mode;
    }
}

/// Infer character fields from user input when the model doesn't produce lock_field calls.
/// This is a safety net — the model should be producing these, but small models sometimes don't.
fn infer_character_fields(user_input: &str, context: &GameContext) -> Vec<ToolCall> {
    let input = user_input.trim();
    if input.len() < 3 {
        return vec![];
    }

    let mut tools = Vec::new();

    // Check which fields are already set by looking at the context's player_office field
    // (which contains the character summary during creation)
    let existing = &context.player_office;

    // Detect background/career keywords
    let bg_keywords = [
        "lawyer", "attorney", "prosecutor", "teacher", "professor", "doctor", "nurse",
        "soldier", "military", "veteran", "business", "entrepreneur", "journalist",
        "engineer", "farmer", "pastor", "minister", "cop", "police", "firefighter",
        "activist", "organizer", "union", "banker", "trader", "scientist",
        "mayor", "senator", "governor", "congressman", "councilman", "councilwoman",
        "boxer", "athlete", "actor", "musician", "writer", "chef", "cook",
    ];
    if !existing.contains("background") {
        let input_lower = input.to_lowercase();
        for kw in &bg_keywords {
            if input_lower.contains(kw) {
                tools.push(ToolCall::LockField {
                    field: "background".into(),
                    value: input.to_string(),
                });
                break;
            }
        }
    }

    // Detect party keywords
    if !existing.contains("party") {
        let input_lower = input.to_lowercase();
        if input_lower.contains("democrat") || input_lower.contains("liberal") || input_lower.contains("progressive") {
            tools.push(ToolCall::LockField {
                field: "party".into(),
                value: "Democrat".into(),
            });
        } else if input_lower.contains("republican") || input_lower.contains("conservative") || input_lower.contains("gop") {
            tools.push(ToolCall::LockField {
                field: "party".into(),
                value: "Republican".into(),
            });
        } else if input_lower.contains("independent") || input_lower.contains("third party") || input_lower.contains("libertarian") || input_lower.contains("green party") {
            tools.push(ToolCall::LockField {
                field: "party".into(),
                value: input.to_string(),
            });
        }
    }

    // Detect motivation keywords
    if !existing.contains("motivation") {
        let input_lower = input.to_lowercase();
        let motivation_keywords = [
            "want to", "driven by", "because", "my goal", "fight for", "change",
            "justice", "revenge", "power", "help people", "make a difference",
            "corruption", "reform",
        ];
        for kw in &motivation_keywords {
            if input_lower.contains(kw) {
                tools.push(ToolCall::LockField {
                    field: "motivation".into(),
                    value: input.to_string(),
                });
                break;
            }
        }
    }

    tools
}

/// Get current timestamp as ISO string (without chrono dependency)
fn chrono_now() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}s", now.as_secs())
}

/// Try to get reasoning text from the DmResponse.
/// The native format parser logs it but doesn't include it in DmResponse.
/// We check if there's reasoning in the tracing output — but actually,
/// we need to get it from the native parser. For now, return None
/// and let the provider log handle it.
fn extract_reasoning_from_response(_response: &super::tools::DmResponse) -> Option<String> {
    // Reasoning is logged by the provider but not in DmResponse.
    // TODO: Add reasoning field to DmResponse or ParsedResponse passthrough
    None
}

fn summarize_tool(tool: &ToolCall) -> String {
    match tool {
        ToolCall::ModifyRel { npc, field, delta } => {
            format!("modify_rel: {} {} {:+}", npc, field, delta)
        }
        ToolCall::GrantCard { card_id, .. } => format!("grant_card: {}", card_id),
        ToolCall::RevokeCard { card_id, .. } => format!("revoke_card: {}", card_id),
        ToolCall::TriggerEvent { event_type, .. } => format!("event: {}", event_type),
        ToolCall::ScheduleEvent {
            event_type,
            weeks_ahead,
            ..
        } => {
            format!("schedule: {} in {}w", event_type, weeks_ahead)
        }
        ToolCall::RollDice { skill, dc, .. } => format!("roll: {} DC{}", skill, dc),
        ToolCall::ScoreAdjust { metric, delta } => format!("score: {} {:+}", metric, delta),
        ToolCall::LockField { field, value } => format!("lock_field: {} = {}", field, value),
        ToolCall::SuggestOptions { field, options, .. } => {
            format!("suggest: {} ({} options)", field, options.len())
        }
        ToolCall::AskQuestion { topic, .. } => format!("ask: {}", topic),
        _ => format!("{:?}", tool).chars().take(40).collect(),
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::mock::MockProvider;

    #[test]
    fn test_agent_turn() {
        let mut agent = Agent::new(DmMode::DungeonMaster);
        let mut provider = MockProvider::new();
        let context = GameContext::default();

        let response = agent.run_turn(
            "I want to meet with Davis",
            &context,
            &mut provider,
            |_tool| None,
            None,
        );

        assert!(!response.narration.is_empty());
        assert_eq!(agent.memory.turn_count(), 1);
        assert!(response.iterations >= 1);
    }

    #[test]
    fn test_agent_memory_accumulates() {
        let mut agent = Agent::new(DmMode::DungeonMaster);
        let mut provider = MockProvider::new();
        let context = GameContext::default();

        for i in 0..5 {
            agent.run_turn(&format!("action {}", i), &context, &mut provider, |_| None, None);
        }

        assert_eq!(agent.memory.turn_count(), 5);
        let history = agent.memory.build_history_block();
        assert!(history.contains("action 0"));
        assert!(history.contains("action 4"));
    }

    #[test]
    fn test_vfs_in_agent() {
        let mut agent = Agent::new(DmMode::DungeonMaster);

        agent
            .vfs
            .write("notebook.md", "Davis is hostile. Watch out.", 1);
        agent
            .vfs
            .write("notes/davis.md", "Rival. Blocked zoning.", 1);

        let context = GameContext {
            active_npcs: vec![crate::ai::context::NpcContext {
                name: "Davis".into(),
                role: "Councilwoman".into(),
                mood: "hostile".into(),
                trust: -20,
                respect: 30,
                recent_memories: vec![],
            }],
            ..GameContext::default()
        };

        let prompt = agent.build_full_prompt("hello Davis", &context);
        assert!(prompt.contains("Davis is hostile"));
        assert!(prompt.contains("Blocked zoning"));
    }

    #[test]
    fn test_prompt_includes_history() {
        let mut agent = Agent::new(DmMode::DungeonMaster);
        let mut provider = MockProvider::new();
        let context = GameContext::default();

        agent.run_turn("hello", &context, &mut provider, |_| None, None);

        let prompt = agent.build_full_prompt("what happened", &context);
        assert!(prompt.contains("hello"));
    }

    #[test]
    fn test_character_creation_mode() {
        let mut agent = Agent::new(DmMode::CharacterCreation);
        let context = GameContext::default();
        let prompt = agent.build_full_prompt("I'm a former prosecutor", &context);
        assert!(prompt.contains("lock_field"));
        assert!(prompt.contains("character"));
    }

    #[test]
    fn test_character_creation_mock_uses_tools() {
        let mut agent = Agent::new(DmMode::CharacterCreation);
        let mut provider = MockProvider::new();
        let context = GameContext::default();

        let response = agent.run_turn(
            "I was a lawyer before this",
            &context,
            &mut provider,
            |_tool| Some("ok".into()),
            None,
        );

        assert!(!response.narration.is_empty());
        // Mock provider should produce tool calls for character creation
        assert!(!response.tool_calls.is_empty());
    }

    #[test]
    fn test_tool_summarization() {
        let tool = ToolCall::ModifyRel {
            npc: "Davis".into(),
            field: "trust".into(),
            delta: -10,
        };
        let summary = summarize_tool(&tool);
        assert_eq!(summary, "modify_rel: Davis trust -10");
    }

    #[test]
    fn test_lock_field_summarization() {
        let tool = ToolCall::LockField {
            field: "background".into(),
            value: "former prosecutor".into(),
        };
        let summary = summarize_tool(&tool);
        assert!(summary.contains("lock_field"));
        assert!(summary.contains("background"));
    }
}
