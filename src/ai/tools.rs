use serde::{Deserialize, Serialize};

/// All tools the AI DM can call to affect the game world
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool", content = "args")]
pub enum ToolCall {
    /// Display narration text to the player
    #[serde(rename = "narrate")]
    Narrate { text: String },

    /// Spawn a new NPC entity
    #[serde(rename = "spawn_npc")]
    SpawnNpc {
        name: String,
        role: Option<String>,
        personality: Option<String>,
    },

    /// Set difficulty class for upcoming roll
    #[serde(rename = "set_dc")]
    SetDc {
        skill: String,
        dc: u32,
        reason: String,
    },

    /// Trigger a game event
    #[serde(rename = "trigger_event")]
    TriggerEvent {
        event_type: String,
        description: String,
    },

    /// Modify a relationship edge
    #[serde(rename = "modify_rel")]
    ModifyRel {
        npc: String,
        field: String,
        delta: i32,
    },

    /// Update a simulation variable
    #[serde(rename = "update_var")]
    UpdateVar { name: String, value: String },

    /// Grant the player a card
    #[serde(rename = "grant_card")]
    GrantCard { card_id: String, reason: String },

    /// Revoke a card from the player
    #[serde(rename = "revoke_card")]
    RevokeCard { card_id: String, reason: String },

    /// Set NPC emotional state
    #[serde(rename = "set_mood")]
    SetMood { npc: String, mood: String },

    /// Trigger a dice roll / skill check
    #[serde(rename = "roll_dice")]
    RollDice {
        skill: String,
        dc: u32,
        modifier: i32,
    },

    /// Schedule a future event
    #[serde(rename = "schedule_event")]
    ScheduleEvent {
        event_type: String,
        description: String,
        weeks_ahead: u32,
    },

    /// Adjust player score/metrics
    #[serde(rename = "score_adjust")]
    ScoreAdjust { metric: String, delta: i32 },

    /// Render a widget inline in the chat (generative UI)
    #[serde(rename = "render_widget")]
    RenderWidget {
        widget_type: WidgetType,
        title: Option<String>,
        data: serde_json::Value,
    },
}

/// Generic widget types the AI can render inline
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    /// Labeled horizontal bars with values
    BarChart,
    /// Single progress bar with label
    Gauge,
    /// Mini trend line from data points
    Sparkline,
    /// Rows and columns with headers
    Table,
    /// Key-value pairs in a bordered box
    StatBlock,
    /// Items with optional icons
    List,
    /// Colored message box (info/warning/success/error)
    Alert,
    /// Attributed text block
    Quote,
    /// Month view with highlighted dates
    Calendar,
}

/// A response from the AI DM: narration text + zero or more tool calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmResponse {
    pub narration: String,
    pub tool_calls: Vec<ToolCall>,
}

/// GBNF grammar for constraining model output to valid tool call JSON.
/// This is the grammar string for constraining ONNX model output via structured decoding.
pub const TOOL_CALL_GBNF: &str = r#"
root ::= "{" ws "\"narration\"" ws ":" ws string ws "," ws "\"tool_calls\"" ws ":" ws "[" ws tool-list ws "]" ws "}"
tool-list ::= "" | tool ("," ws tool)*
tool ::= "{" ws "\"tool\"" ws ":" ws tool-name ws "," ws "\"args\"" ws ":" ws "{" ws args ws "}" ws "}"
tool-name ::= "\"narrate\"" | "\"spawn_npc\"" | "\"set_dc\"" | "\"trigger_event\"" | "\"modify_rel\"" | "\"update_var\"" | "\"grant_card\"" | "\"revoke_card\"" | "\"set_mood\"" | "\"roll_dice\"" | "\"schedule_event\"" | "\"score_adjust\""
args ::= arg ("," ws arg)*
arg ::= string ws ":" ws value
value ::= string | number | "true" | "false" | "null"
string ::= "\"" ([^"\\] | "\\" .)* "\""
number ::= "-"? [0-9]+ ("." [0-9]+)?
ws ::= [ \t\n]*
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_call_serialization() {
        let call = ToolCall::Narrate {
            text: "The room falls silent.".to_string(),
        };
        let json = serde_json::to_string(&call).unwrap();
        assert!(json.contains("narrate"));
        assert!(json.contains("The room falls silent."));
    }

    #[test]
    fn test_tool_call_deserialization() {
        let json = r#"{"tool":"modify_rel","args":{"npc":"Davis","field":"trust","delta":-10}}"#;
        let call: ToolCall = serde_json::from_str(json).unwrap();
        match call {
            ToolCall::ModifyRel { npc, field, delta } => {
                assert_eq!(npc, "Davis");
                assert_eq!(field, "trust");
                assert_eq!(delta, -10);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_dm_response_roundtrip() {
        let response = DmResponse {
            narration: "Davis looks at you skeptically.".to_string(),
            tool_calls: vec![
                ToolCall::SetMood {
                    npc: "Davis".to_string(),
                    mood: "suspicious".to_string(),
                },
                ToolCall::SetDc {
                    skill: "Persuasion".to_string(),
                    dc: 16,
                    reason: "Davis is hostile".to_string(),
                },
            ],
        };

        let json = serde_json::to_string_pretty(&response).unwrap();
        let parsed: DmResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.narration, response.narration);
        assert_eq!(parsed.tool_calls.len(), 2);
    }

    #[test]
    fn test_all_tool_variants_serialize() {
        let calls = vec![
            ToolCall::Narrate {
                text: "test".into(),
            },
            ToolCall::SpawnNpc {
                name: "Test".into(),
                role: None,
                personality: None,
            },
            ToolCall::SetDc {
                skill: "test".into(),
                dc: 10,
                reason: "test".into(),
            },
            ToolCall::TriggerEvent {
                event_type: "crisis".into(),
                description: "test".into(),
            },
            ToolCall::ModifyRel {
                npc: "test".into(),
                field: "trust".into(),
                delta: 5,
            },
            ToolCall::UpdateVar {
                name: "gdp".into(),
                value: "1.5".into(),
            },
            ToolCall::GrantCard {
                card_id: "test".into(),
                reason: "test".into(),
            },
            ToolCall::RevokeCard {
                card_id: "test".into(),
                reason: "test".into(),
            },
            ToolCall::SetMood {
                npc: "test".into(),
                mood: "happy".into(),
            },
            ToolCall::RollDice {
                skill: "test".into(),
                dc: 12,
                modifier: 3,
            },
            ToolCall::ScheduleEvent {
                event_type: "test".into(),
                description: "test".into(),
                weeks_ahead: 3,
            },
            ToolCall::ScoreAdjust {
                metric: "approval".into(),
                delta: 5,
            },
            ToolCall::RenderWidget {
                widget_type: WidgetType::BarChart,
                title: Some("Test Chart".into()),
                data: serde_json::json!({"A": 10, "B": 20}),
            },
        ];

        for call in &calls {
            let json = serde_json::to_string(call).unwrap();
            let _parsed: ToolCall = serde_json::from_str(&json).unwrap();
        }
        assert_eq!(calls.len(), 13); // All 13 tool types
    }
}
