pub mod context;
pub mod mock;
pub mod provider;
pub mod tools;

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

/// The AI harness wraps a provider and handles context building
pub struct AiHarness {
    provider: Box<dyn AiProvider>,
}

impl AiHarness {
    pub fn new(provider: Box<dyn AiProvider>) -> Self {
        Self { provider }
    }

    /// Create with mock provider (for testing / when no model available)
    pub fn mock() -> Self {
        Self {
            provider: Box::new(mock::MockProvider::new()),
        }
    }

    /// Generate a DM response given player input and game context
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
}
