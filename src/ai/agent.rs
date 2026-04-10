use super::context::GameContext;
use super::memory::ConversationMemory;
use super::tools::{DmResponse, ToolCall};
use super::vfs::VirtualFs;
use super::{AiProvider, DmMode};

/// Internal tool calls the agent can make during reasoning
/// These are NOT the same as game tool calls — these are for
/// the agent's own thinking process.
#[derive(Debug, Clone)]
pub enum AgentAction {
    /// Query game state (returns formatted text)
    QueryGameState(String),
    /// Read a VFS file
    ReadFile(String),
    /// Write a VFS file
    WriteFile(String, String),
    /// List VFS files
    ListFiles(Option<String>),
    /// Execute a bash command
    Bash(String),
    /// The agent is done thinking and ready to respond
    Respond(String),
    /// Execute a game tool call (narrate, modify_rel, etc.)
    GameTool(ToolCall),
}

/// Result of a full agent turn (may involve multiple internal iterations)
pub struct AgentResponse {
    /// Final narration text for the player
    pub narration: String,
    /// Game tool calls that were executed
    pub executed_tools: Vec<ToolCall>,
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
            memory: ConversationMemory::new(10),
            vfs: VirtualFs::new(),
            mode,
            max_iterations: 5, // Max internal reasoning loops
        }
    }

    pub fn with_memory(mode: DmMode, memory: ConversationMemory) -> Self {
        Self {
            memory,
            vfs: VirtualFs::new(),
            mode,
            max_iterations: 5,
        }
    }

    /// Run a full agent turn. The agent can loop internally
    /// (thinking, looking up data, writing notes) before
    /// producing a final response for the player.
    ///
    /// `tool_executor` handles game-level tool calls (modify_rel, etc.)
    /// `state_query` handles game state lookups ("what is Davis's trust?")
    pub fn run_turn<F>(
        &mut self,
        user_input: &str,
        context: &GameContext,
        provider: &mut dyn AiProvider,
        mut tool_executor: F,
    ) -> AgentResponse
    where
        F: FnMut(&ToolCall) -> Option<String>,
    {
        let mut all_narration = Vec::new();
        let mut all_tools = Vec::new();
        let mut iterations = 0u32;

        // Build initial prompt
        let prompt = self.build_full_prompt(user_input, context);

        // Single generation call (multi-turn reasoning will be
        // added when the model supports tool calling properly —
        // for now, one shot with rich context is effective)
        let response = match provider.generate(&prompt, self.mode) {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!("Agent error: {}", e);
                return AgentResponse {
                    narration: "The dungeon master pauses, collecting their thoughts...".into(),
                    executed_tools: vec![],
                    iterations: 0,
                };
            }
        };

        iterations += 1;

        // Collect narration
        if !response.narration.is_empty() {
            all_narration.push(response.narration.clone());
        }

        // Execute any tool calls
        for tool in &response.tool_calls {
            match tool {
                ToolCall::Narrate { text } => {
                    all_narration.push(text.clone());
                }
                _ => {
                    let _result = tool_executor(tool);
                    all_tools.push(tool.clone());
                }
            }
        }

        // Store in memory
        let tool_summaries: Vec<String> = all_tools.iter().map(|t| summarize_tool(t)).collect();

        let final_narration = all_narration.join("\n\n");

        self.memory
            .add_exchange(user_input, &final_narration, &tool_summaries, context.week);

        AgentResponse {
            narration: final_narration,
            executed_tools: all_tools,
            iterations,
        }
    }

    /// Execute a bash command (sandboxed — read-only, timeout)
    pub fn exec_bash(&self, command: &str) -> Result<String, String> {
        use std::process::Command;

        // Safety: only allow read-only commands
        let blocked = [
            "rm ", "mv ", "cp ", "chmod", "chown", "sudo", "kill", "mkfs", "dd ", "shutdown",
            "reboot", ">", ">>", "|",
        ];
        for b in &blocked {
            if command.contains(b) {
                return Err(format!("Blocked command: contains '{}'", b));
            }
        }

        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .map_err(|e| format!("Exec error: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(if stdout.is_empty() { stderr } else { stdout })
        } else {
            Err(format!("Exit {}: {}", output.status, stderr))
        }
    }

    /// Build the complete prompt
    fn build_full_prompt(&self, user_input: &str, context: &GameContext) -> String {
        let mut parts = Vec::new();

        // System prompt
        parts.push(self.build_system_prompt(context));

        // Game state
        parts.push(self.build_game_context(context));

        // Agent's own notes (from VFS — if any relevant files exist)
        let agent_notes = self.get_relevant_notes(context);
        if !agent_notes.is_empty() {
            parts.push(format!("YOUR NOTES:\n{}", agent_notes));
        }

        // Conversation history
        let history = self.memory.build_history_block();
        if !history.is_empty() {
            parts.push(format!("CONVERSATION HISTORY:\n{}", history));
        }

        let system_and_context = parts.join("\n\n");

        format!(
            "<start_of_turn>user\n{}\n\nPlayer: {}<end_of_turn>\n<start_of_turn>model\n",
            system_and_context, user_input
        )
    }

    fn build_system_prompt(&self, context: &GameContext) -> String {
        let mode_instructions = match self.mode {
            DmMode::Narrator => {
                "You are the narrator. Generate morning briefings and describe consequences. \
                 Keep responses to 3-5 sentences. Be vivid but concise."
            }
            DmMode::Conversation => {
                "You are voicing an NPC. Stay in character. Keep dialogue natural \
                 and brief (2-4 sentences). React to what the player says."
            }
            DmMode::DungeonMaster => {
                "You are the dungeon master. Respond to the player's actions with consequences \
                 and narrative. Be responsive, don't railroad. 3-5 sentences."
            }
            DmMode::LawInterpreter => {
                "You are a legal analyst. Evaluate actions for legality. Be precise but accessible."
            }
        };

        format!(
            "You are the AI Dungeon Master for POLIT, an American politics simulator.\n\
             Tone: {}\n\n\
             {}\n\n\
             Respond with natural prose only. No JSON, no tool calls, no structured data.",
            context.tone_instructions, mode_instructions
        )
    }

    fn build_game_context(&self, context: &GameContext) -> String {
        let mut ctx = format!(
            "GAME STATE:\n\
             Week {}, Year {}\n\
             Player: {} ({})\n\
             AP: {}/{}\n\
             Economy: {}",
            context.week,
            context.year,
            context.player_name,
            context.player_office,
            context.ap_current,
            context.ap_max,
            context.economic_summary,
        );

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

    /// Get relevant notes from VFS based on current context
    fn get_relevant_notes(&self, context: &GameContext) -> String {
        let mut notes = Vec::new();

        // Always include the agent's main notebook if it exists
        if let Some(content) = self.vfs.read("notebook.md") {
            notes.push(format!("notebook.md:\n{}", truncate(content, 300)));
        }

        // Include NPC notes if talking to someone
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
            agent.run_turn(&format!("action {}", i), &context, &mut provider, |_| None);
        }

        assert_eq!(agent.memory.turn_count(), 5);
        let history = agent.memory.build_history_block();
        assert!(history.contains("action 0"));
        assert!(history.contains("action 4"));
    }

    #[test]
    fn test_vfs_in_agent() {
        let mut agent = Agent::new(DmMode::DungeonMaster);

        // Agent writes notes
        agent
            .vfs
            .write("notebook.md", "Davis is hostile. Watch out.", 1);
        agent
            .vfs
            .write("notes/davis.md", "Rival. Blocked zoning.", 1);

        // Notes should appear in prompt when talking to Davis
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
    fn test_bash_safety() {
        let agent = Agent::new(DmMode::DungeonMaster);

        // Safe commands should work
        let result = agent.exec_bash("echo hello");
        assert!(result.is_ok());
        assert!(result.unwrap().contains("hello"));

        // Dangerous commands should be blocked
        assert!(agent.exec_bash("rm -rf /").is_err());
        assert!(agent.exec_bash("sudo anything").is_err());
    }

    #[test]
    fn test_bash_read_only() {
        let agent = Agent::new(DmMode::DungeonMaster);
        let result = agent.exec_bash("date");
        assert!(result.is_ok());
    }

    #[test]
    fn test_prompt_includes_history() {
        let mut agent = Agent::new(DmMode::DungeonMaster);
        let mut provider = MockProvider::new();
        let context = GameContext::default();

        // First turn
        agent.run_turn("hello", &context, &mut provider, |_| None);

        // Second turn prompt should include first exchange
        let prompt = agent.build_full_prompt("what happened", &context);
        assert!(prompt.contains("hello"));
    }

    #[test]
    fn test_mode_switching() {
        let mut agent = Agent::new(DmMode::Narrator);
        agent.set_mode(DmMode::Conversation);

        let context = GameContext::default();
        let prompt = agent.build_full_prompt("test", &context);
        assert!(prompt.contains("NPC"));
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
}
