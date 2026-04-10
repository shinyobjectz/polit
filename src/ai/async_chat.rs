use crossbeam_channel::{bounded, Receiver, Sender};
use std::thread;

use super::tools::DmResponse;
use super::{AiProvider, DmMode};

/// Messages from UI → AI thread
pub enum AiRequest {
    Generate { prompt: String, mode: DmMode },
    Shutdown,
}

/// Messages from AI thread → UI
pub enum AiResponse {
    /// AI is working (show typing animation)
    Thinking,
    /// AI finished — here's the response
    Done(DmResponse),
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
    /// Spawn the AI thread with the given provider
    pub fn new(mut provider: Box<dyn AiProvider>) -> Self {
        let (request_tx, request_rx) = bounded::<AiRequest>(4);
        let (response_tx, response_rx) = bounded::<AiResponse>(4);

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

    /// Send a generation request (non-blocking)
    pub fn request_generation(&mut self, prompt: &str, mode: DmMode) {
        self.is_thinking = true;
        let _ = self.request_tx.send(AiRequest::Generate {
            prompt: prompt.to_string(),
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
