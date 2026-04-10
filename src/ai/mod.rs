pub mod agent;
pub mod async_chat;
pub mod context;
pub mod memory;
pub mod mock;
pub mod provider;
pub mod tools;
pub mod vfs;

use tools::DmResponse;

/// The DM mode determines how context is built and what tools are prioritized
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmMode {
    /// Between actions — briefings, consequences
    Narrator,
    /// Player talking to NPCs
    Conversation,
    /// Setting up and adjudicating events
    DungeonMaster,
    /// Checking law compliance
    LawInterpreter,
}

/// Trait for AI providers (real model or mock)
pub trait AiProvider: Send {
    fn generate(
        &mut self,
        prompt: &str,
        mode: DmMode,
    ) -> Result<DmResponse, Box<dyn std::error::Error + Send + Sync>>;
    fn name(&self) -> &str;
}

/// The AI harness wraps a provider and the agent orchestrator.
/// This is the main interface for the game to interact with the AI.
pub struct AiHarness {
    pub provider: Box<dyn AiProvider>,
    pub agent: agent::Agent,
}

impl AiHarness {
    pub fn new(provider: Box<dyn AiProvider>) -> Self {
        Self {
            provider,
            agent: agent::Agent::new(DmMode::DungeonMaster),
        }
    }

    pub fn mock() -> Self {
        Self {
            provider: Box::new(mock::MockProvider::new()),
            agent: agent::Agent::new(DmMode::DungeonMaster),
        }
    }

    /// Run a full agent turn with tool execution
    pub fn run_turn<F>(
        &mut self,
        player_input: &str,
        context: &context::GameContext,
        mode: DmMode,
        tool_executor: F,
    ) -> agent::AgentResponse
    where
        F: FnMut(&tools::ToolCall) -> Option<String>,
    {
        self.agent.set_mode(mode);
        self.agent
            .run_turn(player_input, context, self.provider.as_mut(), tool_executor)
    }

    /// Simple generate without tool loop (for backward compat)
    pub fn respond(
        &mut self,
        player_input: &str,
        context: &context::GameContext,
        mode: DmMode,
    ) -> Result<DmResponse, Box<dyn std::error::Error + Send + Sync>> {
        let prompt = context.build_prompt(player_input, mode);
        self.provider.generate(&prompt, mode)
    }

    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }

    pub fn memory(&self) -> &memory::ConversationMemory {
        &self.agent.memory
    }
}
