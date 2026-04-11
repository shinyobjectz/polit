use crossbeam_channel::{bounded, Receiver, Sender};
use std::thread;

use super::agent::{Agent, AgentResponse};
use super::context::GameContext;
use super::tools::DmResponse;
use super::{AiProvider, DmMode};

/// Messages from UI → AI thread
pub enum AiRequest {
    /// Simple one-shot generation (backward compat)
    Generate { prompt: String, mode: DmMode },
    /// Full agent turn with memory, tools, and context
    AgentTurn {
        user_input: String,
        context: GameContext,
        mode: DmMode,
    },
    Shutdown,
}

/// A step the agent completed (streamed to UI in real-time)
#[derive(Debug, Clone)]
pub enum AgentStep {
    /// Agent is reasoning internally
    Thinking(String),
    /// Agent executed a tool
    ToolExecuted(String),
    /// Agent is generating (iteration N)
    Generating(u32),
}

/// Messages from AI thread → UI
pub enum AiResponse {
    /// AI is working (show typing animation)
    Thinking,
    /// Intermediate step completed (show in chat as it happens)
    Step(AgentStep),
    /// Simple generation finished
    Done(DmResponse),
    /// Full agent turn finished (narration + tool calls)
    AgentDone(AgentResponse),
    /// AI had an error
    Error(String),
}

/// Async AI chat — runs inference on a background thread,
/// communicates via channels so UI never blocks.
pub struct AsyncAiChat {
    request_tx: Sender<AiRequest>,
    response_rx: Receiver<AiResponse>,
    _handle: thread::JoinHandle<()>,
    pub is_thinking: bool,
}

impl AsyncAiChat {
    /// Spawn the AI thread with the given provider.
    pub fn new(provider: Box<dyn AiProvider>) -> Self {
        Self::with_agent(provider, Agent::new(DmMode::DungeonMaster))
    }

    /// Spawn with a specific agent (e.g., character creation mode)
    pub fn with_agent(mut provider: Box<dyn AiProvider>, mut agent: Agent) -> Self {
        let (request_tx, request_rx) = bounded::<AiRequest>(4);
        let (response_tx, response_rx) = bounded::<AiResponse>(16); // larger buffer for steps

        let handle = thread::Builder::new()
            .name("polit-ai".to_string())
            .spawn(move || loop {
                match request_rx.recv() {
                    Ok(AiRequest::Generate { prompt, mode }) => {
                        let _ = response_tx.send(AiResponse::Thinking);
                        match provider.generate(&prompt, mode) {
                            Ok(response) => {
                                let _ = response_tx.send(AiResponse::Done(response));
                            }
                            Err(e) => {
                                let _ = response_tx.send(AiResponse::Error(format!("{}", e)));
                            }
                        }
                    }
                    Ok(AiRequest::AgentTurn {
                        user_input,
                        context,
                        mode,
                    }) => {
                        let _ = response_tx.send(AiResponse::Thinking);
                        agent.set_mode(mode);

                        // Pass a step sender so the agent can stream progress
                        let step_tx = response_tx.clone();
                        let response = agent.run_turn(
                            &user_input,
                            &context,
                            provider.as_mut(),
                            |_tool| None,
                            Some(&move |step: AgentStep| {
                                let _ = step_tx.send(AiResponse::Step(step));
                            }),
                        );
                        let _ = response_tx.send(AiResponse::AgentDone(response));
                    }
                    Ok(AiRequest::Shutdown) | Err(_) => break,
                }
            })
            .expect("Failed to spawn AI thread");

        Self {
            request_tx,
            response_rx,
            _handle: handle,
            is_thinking: false,
        }
    }

    /// Send a simple generation request (non-blocking)
    pub fn request_generation(&mut self, prompt: &str, mode: DmMode) {
        self.is_thinking = true;
        let _ = self.request_tx.send(AiRequest::Generate {
            prompt: prompt.to_string(),
            mode,
        });
    }

    /// Send a full agent turn request (non-blocking).
    pub fn request_agent_turn(&mut self, user_input: &str, context: GameContext, mode: DmMode) {
        self.is_thinking = true;
        let _ = self.request_tx.send(AiRequest::AgentTurn {
            user_input: user_input.to_string(),
            context,
            mode,
        });
    }

    /// Poll for response (non-blocking). Returns None if still thinking.
    pub fn poll_response(&mut self) -> Option<AiResponse> {
        match self.response_rx.try_recv() {
            Ok(AiResponse::Thinking) => {
                self.is_thinking = true;
                None
            }
            Ok(resp @ AiResponse::Step(_)) => {
                // Steps don't end the thinking state
                Some(resp)
            }
            Ok(resp) => {
                self.is_thinking = false;
                Some(resp)
            }
            Err(_) => None,
        }
    }

    /// Shutdown the AI thread
    pub fn shutdown(&self) {
        let _ = self.request_tx.send(AiRequest::Shutdown);
    }
}

impl Drop for AsyncAiChat {
    fn drop(&mut self) {
        self.shutdown();
    }
}
