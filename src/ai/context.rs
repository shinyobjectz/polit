use super::DmMode;

/// Game context assembled for each AI call
#[derive(Debug, Clone)]
pub struct GameContext {
    pub week: u32,
    pub year: u32,
    pub phase: String,
    pub player_name: String,
    pub player_office: String,
    pub ap_current: i32,
    pub ap_max: i32,
    pub active_npcs: Vec<NpcContext>,
    pub recent_events: Vec<String>,
    pub economic_summary: String,
    pub tone_instructions: String,
}

#[derive(Debug, Clone)]
pub struct NpcContext {
    pub name: String,
    pub role: String,
    pub mood: String,
    pub trust: i32,
    pub respect: i32,
    pub recent_memories: Vec<String>,
}

impl Default for GameContext {
    fn default() -> Self {
        Self {
            week: 1,
            year: 2024,
            phase: "Action".into(),
            player_name: "Player".into(),
            player_office: "City Council Member".into(),
            ap_current: 5,
            ap_max: 5,
            active_npcs: vec![],
            recent_events: vec![],
            economic_summary: "Economy is stable.".into(),
            tone_instructions: "Realistic political drama. Serious but not dry.".into(),
        }
    }
}

impl GameContext {
    /// Build the full prompt for the AI model using Gemma 4 native format
    pub fn build_prompt(&self, player_input: &str, mode: DmMode) -> String {
        let system = self.build_system_prompt(mode);
        let context = self.build_context_block();
        let full_system = format!("{}\n\n{}", system, context);
        let tools = super::native_format::tool_declarations(mode);
        let player_msg = format!("Player says: {}", player_input);

        super::native_format::build_prompt(&full_system, &tools, &[], &player_msg)
    }

    fn build_system_prompt(&self, mode: DmMode) -> String {
        let mode_instructions = match mode {
            DmMode::Narrator => {
                "You are the narrator of a political simulation game. Generate a morning briefing \
                 or describe consequences of player actions. Use narrate() tool calls for text output. \
                 Use schedule_event() for upcoming developments. Use update_var() for simulation changes."
            }
            DmMode::Conversation => {
                "You are voicing an NPC in a political simulation. Stay in character based on \
                 the NPC's personality, goals, and relationship with the player. Use narrate() for \
                 dialogue. Use modify_rel() if the conversation shifts the relationship. Use set_mood() \
                 to reflect the NPC's emotional state. Use roll_dice() if the player attempts persuasion."
            }
            DmMode::DungeonMaster => {
                "You are the dungeon master of a political simulation. Set up dramatic situations, \
                 determine difficulty classes for skill checks, and create branching narrative moments. \
                 Use set_dc() for upcoming rolls. Use trigger_event() for dramatic developments. \
                 Use grant_card() or revoke_card() for milestone moments."
            }
            DmMode::LawInterpreter => {
                "You are a legal analyst in a political simulation. Evaluate whether proposed actions \
                 comply with active laws. Convert player intentions into legal language. Flag \
                 constitutional issues. Use narrate() for legal analysis."
            }
            DmMode::CharacterCreation => {
                "You are collaboratively building a character with the player. \
                 Use lock_field() to record character details the player reveals. \
                 Use ask_question() to explore new character aspects. \
                 Use suggest_options() to offer choices. Do NOT describe physical \
                 locations or set scenes. Stay conversational. 2-4 sentences."
            }
        };

        format!(
            "You are the AI Dungeon Master for POLIT, an American politics simulator.\n\
             Tone: {}\n\n\
             {}\n\n\
             Respond with a JSON object containing:\n\
             - \"narration\": string with your narrative text\n\
             - \"tool_calls\": array of tool call objects\n\n\
             Available tools: narrate, spawn_npc, set_dc, trigger_event, modify_rel, \
             update_var, grant_card, revoke_card, set_mood, roll_dice, schedule_event, score_adjust",
            self.tone_instructions, mode_instructions
        )
    }

    fn build_context_block(&self) -> String {
        let mut ctx = format!(
            "GAME STATE:\n\
             Week {}, Year {}\n\
             Player: {} ({})\n\
             AP: {}/{}\n\
             Phase: {}\n\
             Economy: {}",
            self.week,
            self.year,
            self.player_name,
            self.player_office,
            self.ap_current,
            self.ap_max,
            self.phase,
            self.economic_summary,
        );

        if !self.active_npcs.is_empty() {
            ctx.push_str("\n\nNPCs IN SCENE:");
            for npc in &self.active_npcs {
                ctx.push_str(&format!(
                    "\n  {} ({}) — mood: {}, trust: {}, respect: {}",
                    npc.name, npc.role, npc.mood, npc.trust, npc.respect
                ));
                for mem in &npc.recent_memories {
                    ctx.push_str(&format!("\n    memory: {}", mem));
                }
            }
        }

        if !self.recent_events.is_empty() {
            ctx.push_str("\n\nRECENT EVENTS:");
            for event in &self.recent_events {
                ctx.push_str(&format!("\n  - {}", event));
            }
        }

        ctx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_context() {
        let ctx = GameContext::default();
        assert_eq!(ctx.week, 1);
        assert_eq!(ctx.player_name, "Player");
    }

    #[test]
    fn test_prompt_contains_player_input() {
        let ctx = GameContext::default();
        let prompt = ctx.build_prompt("I want to meet with Davis", DmMode::Conversation);
        assert!(prompt.contains("I want to meet with Davis"));
        assert!(prompt.contains("model"));
    }

    #[test]
    fn test_prompt_mode_instructions() {
        let ctx = GameContext::default();

        let narrator = ctx.build_prompt("test", DmMode::Narrator);
        assert!(narrator.contains("narrator"));

        let convo = ctx.build_prompt("test", DmMode::Conversation);
        assert!(convo.contains("NPC"));

        let dm = ctx.build_prompt("test", DmMode::DungeonMaster);
        assert!(dm.contains("dungeon master"));

        let law = ctx.build_prompt("test", DmMode::LawInterpreter);
        assert!(law.contains("legal"));
    }

    #[test]
    fn test_context_with_npcs() {
        let ctx = GameContext {
            active_npcs: vec![NpcContext {
                name: "Davis".into(),
                role: "Councilwoman".into(),
                mood: "suspicious".into(),
                trust: -20,
                respect: 30,
                recent_memories: vec!["Blocked your zoning proposal".into()],
            }],
            ..Default::default()
        };
        let prompt = ctx.build_prompt("Hello", DmMode::Conversation);
        assert!(prompt.contains("Davis"));
        assert!(prompt.contains("suspicious"));
        assert!(prompt.contains("Blocked your zoning"));
    }

    #[test]
    fn test_prompt_token_budget() {
        // Ensure default prompt stays under ~6000 chars (roughly <1500 tokens)
        // Native format with tool declarations is larger than old JSON format
        let ctx = GameContext::default();
        let prompt = ctx.build_prompt("test input", DmMode::Narrator);
        assert!(
            prompt.len() < 6000,
            "Prompt too long: {} chars",
            prompt.len()
        );
    }
}
