use rand::Rng;

use super::tools::{DmResponse, ToolCall};
use super::{AiProvider, DmMode};

/// Mock AI provider that generates deterministic but varied DM responses
/// Used for testing and when no ONNX model is available
pub struct MockProvider {
    call_count: u32,
}

impl MockProvider {
    pub fn new() -> Self {
        Self { call_count: 0 }
    }
}

impl AiProvider for MockProvider {
    fn name(&self) -> &str {
        "mock-dm"
    }

    fn generate(
        &mut self,
        prompt: &str,
        mode: DmMode,
    ) -> Result<DmResponse, Box<dyn std::error::Error + Send + Sync>> {
        self.call_count += 1;
        let mut rng = rand::thread_rng();

        let response = match mode {
            DmMode::Narrator => generate_narration(&mut rng, self.call_count),
            DmMode::Conversation => generate_conversation(&mut rng, prompt),
            DmMode::DungeonMaster => generate_dm_event(&mut rng),
            DmMode::LawInterpreter => generate_legal(&mut rng, prompt),
            DmMode::CharacterCreation => generate_creation(&mut rng, prompt, self.call_count),
        };

        Ok(response)
    }
}

fn generate_narration(rng: &mut impl Rng, week: u32) -> DmResponse {
    let briefings = [
        "The morning paper leads with rising tensions over the zoning debate. Your constituents are watching closely.",
        "Poll numbers are in — your approval ticked up 3 points after last week's town hall. The opposition is regrouping.",
        "A cold front swept through the district overnight. More concerning: so did a rumor about budget mismanagement in the parks department.",
        "The business association released their quarterly report. Job growth is modest but steady. They want to meet.",
        "Your chief of staff flagged three items: a reporter asking about infrastructure delays, a donor requesting face time, and a school board dispute that's heating up.",
        "Quiet week on the surface, but your sources say Councilwoman Davis is building a coalition against your housing initiative.",
        "Federal grants were announced this morning. Your district is eligible for $2M in infrastructure funding — if you can put together a proposal by month's end.",
        "A constituent stopped you at the grocery store — their small business is struggling with the new parking regulations you supported. It stuck with you.",
    ];

    let idx = (week as usize) % briefings.len();
    let mut tool_calls = vec![];

    // Sometimes schedule a future event
    if rng.gen_bool(0.3) {
        tool_calls.push(ToolCall::ScheduleEvent {
            event_type: "opportunity".into(),
            description: "A local business leader wants to discuss a partnership.".into(),
            weeks_ahead: rng.gen_range(1..=4),
        });
    }

    DmResponse {
        narration: briefings[idx].to_string(),
        tool_calls,
    }
}

fn generate_conversation(rng: &mut impl Rng, prompt: &str) -> DmResponse {
    let input_lower = prompt.to_lowercase();
    let mut tool_calls = vec![];

    let narration = if input_lower.contains("deal")
        || input_lower.contains("offer")
        || input_lower.contains("support")
    {
        tool_calls.push(ToolCall::ModifyRel {
            npc: "current_npc".into(),
            field: "trust".into(),
            delta: if rng.gen_bool(0.6) { 5 } else { -3 },
        });
        "They lean forward, considering your proposal. \"That's... interesting. I'd need to see the details, \
         but I'm not opposed in principle. What exactly would you need from me?\""
    } else if input_lower.contains("threaten")
        || input_lower.contains("pressure")
        || input_lower.contains("demand")
    {
        tool_calls.push(ToolCall::ModifyRel {
            npc: "current_npc".into(),
            field: "fear".into(),
            delta: 10,
        });
        tool_calls.push(ToolCall::ModifyRel {
            npc: "current_npc".into(),
            field: "trust".into(),
            delta: -15,
        });
        tool_calls.push(ToolCall::SetMood {
            npc: "current_npc".into(),
            mood: "hostile".into(),
        });
        "Their expression hardens. \"I'd choose my next words very carefully if I were you. \
         People in this town have long memories.\""
    } else if input_lower.contains("help")
        || input_lower.contains("favor")
        || input_lower.contains("ask")
    {
        tool_calls.push(ToolCall::RollDice {
            skill: "Persuasion".into(),
            dc: 12,
            modifier: 0,
        });
        "They sigh and look out the window. \"You know I've always tried to be fair with you. \
         Let me think about it — I'll have an answer by end of week.\""
    } else {
        tool_calls.push(ToolCall::ModifyRel {
            npc: "current_npc".into(),
            field: "knowledge".into(),
            delta: 3,
        });
        "They nod thoughtfully, taking in what you've said. \"I appreciate you being \
         straight with me. Not everyone in this building does that.\""
    };

    DmResponse {
        narration: narration.to_string(),
        tool_calls,
    }
}

fn generate_dm_event(rng: &mut impl Rng) -> DmResponse {
    let events = [
        (
            "A local factory just announced layoffs — 200 jobs gone by end of quarter. \
             The media is already at your door.",
            ToolCall::TriggerEvent {
                event_type: "crisis".into(),
                description: "Factory layoffs".into(),
            },
        ),
        (
            "Breaking: a water main burst on 5th Avenue. Three blocks are flooded. \
             Emergency services are stretched thin.",
            ToolCall::TriggerEvent {
                event_type: "crisis".into(),
                description: "Water main break".into(),
            },
        ),
        (
            "Good news for once — a tech company is scouting your district for a new office. \
             They want to meet with local leadership.",
            ToolCall::ScheduleEvent {
                event_type: "opportunity".into(),
                description: "Tech company meeting".into(),
                weeks_ahead: 2,
            },
        ),
        (
            "The state party chair called. They're considering you for a speaking slot \
             at the regional convention. This could raise your profile significantly.",
            ToolCall::GrantCard {
                card_id: "convention_speaker".into(),
                reason: "Party recognition".into(),
            },
        ),
    ];

    let (narration, tool_call) = &events[rng.gen_range(0..events.len())];
    DmResponse {
        narration: narration.to_string(),
        tool_calls: vec![tool_call.clone()],
    }
}

fn generate_creation(_rng: &mut impl Rng, prompt: &str, call_count: u32) -> DmResponse {
    let input_lower = prompt.to_lowercase();
    let mut tool_calls = vec![];

    let narration = match call_count {
        1 => {
            tool_calls.push(ToolCall::AskQuestion {
                topic: "background".into(),
                question: "What have you done with your life so far?".into(),
            });
            "Welcome! I'm excited to learn who you are. Tell me — before all this, \
             before politics, what was your life? What did you do, and what made you \
             want something different?"
        }
        2 => {
            if input_lower.contains("lawyer") || input_lower.contains("attorney") || input_lower.contains("prosecutor") {
                tool_calls.push(ToolCall::LockField {
                    field: "background".into(),
                    value: "Legal professional".into(),
                });
            } else if input_lower.contains("business") || input_lower.contains("company") {
                tool_calls.push(ToolCall::LockField {
                    field: "background".into(),
                    value: "Business executive".into(),
                });
            } else if input_lower.contains("teacher") || input_lower.contains("professor") {
                tool_calls.push(ToolCall::LockField {
                    field: "background".into(),
                    value: "Educator".into(),
                });
            }
            tool_calls.push(ToolCall::AskQuestion {
                topic: "motivation".into(),
                question: "What drives you into public life?".into(),
            });
            "Interesting. That tells me a lot about how you see the world. \
             So what's the thing that finally pushed you into the arena? \
             Was there a moment, an injustice, a promise you made?"
        }
        _ => {
            tool_calls.push(ToolCall::AskQuestion {
                topic: "general".into(),
                question: "Tell me more about yourself.".into(),
            });
            "That's fascinating — I can see the shape of who you are forming. \
             Tell me something nobody else knows about you. What's the thing \
             you'd never put on a campaign poster?"
        }
    };

    DmResponse {
        narration: narration.to_string(),
        tool_calls,
    }
}

fn generate_legal(rng: &mut impl Rng, prompt: &str) -> DmResponse {
    let narration = format!(
        "Legal analysis of your proposal:\n\n\
         The action you've described falls under municipal code section 4.2.1. \
         Based on current ordinances, this would require a simple majority vote \
         of the council. No constitutional issues identified, though Councilwoman \
         Davis may challenge it on procedural grounds.\n\n\
         Recommendation: Proceed with a formal proposal at next week's session."
    );

    DmResponse {
        narration,
        tool_calls: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_generates_responses() {
        let mut provider = MockProvider::new();

        for mode in [
            DmMode::Narrator,
            DmMode::Conversation,
            DmMode::DungeonMaster,
            DmMode::LawInterpreter,
        ] {
            let response = provider.generate("test input", mode).unwrap();
            assert!(!response.narration.is_empty());
        }
    }

    #[test]
    fn test_conversation_reacts_to_threats() {
        let mut provider = MockProvider::new();
        let response = provider
            .generate("I will threaten you", DmMode::Conversation)
            .unwrap();
        assert!(response.narration.contains("carefully"));
        // Should have fear + trust modifiers
        assert!(response.tool_calls.len() >= 2);
    }

    #[test]
    fn test_conversation_reacts_to_deals() {
        let mut provider = MockProvider::new();
        let response = provider
            .generate("Let's make a deal", DmMode::Conversation)
            .unwrap();
        assert!(
            response.narration.contains("proposal") || response.narration.contains("interesting")
        );
    }

    #[test]
    fn test_dm_events_have_tool_calls() {
        let mut provider = MockProvider::new();
        let response = provider.generate("", DmMode::DungeonMaster).unwrap();
        assert!(!response.tool_calls.is_empty());
    }

    #[test]
    fn test_narrator_varies_by_week() {
        let mut provider = MockProvider::new();
        let r1 = provider.generate("", DmMode::Narrator).unwrap();
        let r2 = provider.generate("", DmMode::Narrator).unwrap();
        // Call count differs, so briefings should differ
        // (they use call_count % briefings.len())
        assert_ne!(r1.narration, r2.narration);
    }
}
