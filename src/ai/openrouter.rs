use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::provider::parse_dm_response;
use super::secrets::{load_openrouter_api_key, SecureStorage};
use super::tools::DmResponse;
use super::{AiProvider, DmMode};

const OPENROUTER_CHAT_COMPLETIONS_URL: &str =
    "https://openrouter.ai/api/v1/chat/completions";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenRouterError {
    MissingModel(String),
    MissingApiKey(String),
    ValidationFailed(String),
    RequestFailed(String),
    EmptyResponse,
}

impl fmt::Display for OpenRouterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenRouterError::MissingModel(message)
            | OpenRouterError::MissingApiKey(message)
            | OpenRouterError::ValidationFailed(message)
            | OpenRouterError::RequestFailed(message) => f.write_str(message),
            OpenRouterError::EmptyResponse => f.write_str("openrouter returned an empty response"),
        }
    }
}

impl std::error::Error for OpenRouterError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenRouterRequestMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenRouterRequest {
    pub model: String,
    pub messages: Vec<OpenRouterRequestMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenRouterMessage {
    pub content: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenRouterChoice {
    pub message: OpenRouterMessage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenRouterResponse {
    pub choices: Vec<OpenRouterChoice>,
}

pub trait OpenRouterClient: Send {
    fn chat_completions(
        &self,
        api_key: &str,
        request: OpenRouterRequest,
    ) -> Result<OpenRouterResponse, OpenRouterError>;
}

#[derive(Debug, Clone)]
pub struct ReqwestOpenRouterClient {
    http: reqwest::blocking::Client,
}

impl Default for ReqwestOpenRouterClient {
    fn default() -> Self {
        let http = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .build()
            .expect("failed to build reqwest blocking client");
        Self {
            http,
        }
    }
}

impl OpenRouterClient for ReqwestOpenRouterClient {
    fn chat_completions(
        &self,
        api_key: &str,
        request: OpenRouterRequest,
    ) -> Result<OpenRouterResponse, OpenRouterError> {
        let response = self
            .http
            .post(OPENROUTER_CHAT_COMPLETIONS_URL)
            .bearer_auth(api_key)
            .json(&request)
            .send()
            .map_err(|error: reqwest::Error| OpenRouterError::RequestFailed(error.to_string()))?;

        let response = response
            .error_for_status()
            .map_err(|error: reqwest::Error| OpenRouterError::RequestFailed(error.to_string()))?;

        response
            .json::<OpenRouterResponse>()
            .map_err(|error: reqwest::Error| OpenRouterError::RequestFailed(error.to_string()))
    }
}

pub struct OpenRouterProvider<C = ReqwestOpenRouterClient> {
    model: String,
    api_key: String,
    client: C,
}

impl<C> fmt::Debug for OpenRouterProvider<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenRouterProvider")
            .field("model", &self.model)
            .field("api_key", &"<redacted>")
            .finish_non_exhaustive()
    }
}

impl OpenRouterProvider<ReqwestOpenRouterClient> {
    pub fn system(
        model: Option<&str>,
        storage: &dyn SecureStorage,
    ) -> Result<Self, OpenRouterError> {
        Self::new(model, storage, ReqwestOpenRouterClient::default())
    }
}

impl<C: OpenRouterClient> OpenRouterProvider<C> {
    pub fn new(
        model: Option<&str>,
        storage: &dyn SecureStorage,
        client: C,
    ) -> Result<Self, OpenRouterError> {
        let model = model
            .map(str::to_string)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                OpenRouterError::MissingModel(
                    "openrouter model is required before startup validation".to_string(),
                )
            })?;

        let api_key = load_openrouter_api_key(storage)
            .map_err(|error| OpenRouterError::ValidationFailed(error.to_string()))?
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                OpenRouterError::MissingApiKey(
                    "openrouter api key is required before startup validation".to_string(),
                )
            })?;

        let validation_request = OpenRouterRequest {
            model: model.clone(),
            messages: vec![OpenRouterRequestMessage {
                role: "user".to_string(),
                content: "Reply with a short confirmation that the model is available."
                    .to_string(),
            }],
        };

        let validation_response = client
            .chat_completions(&api_key, validation_request)
            .map_err(|error| OpenRouterError::ValidationFailed(error.to_string()))?;

        let _ = extract_response_content(&validation_response)?;

        Ok(Self {
            model,
            api_key,
            client,
        })
    }

    fn chat_completion(
        &self,
        prompt: &str,
    ) -> Result<OpenRouterResponse, OpenRouterError> {
        let request = OpenRouterRequest {
            model: self.model.clone(),
            messages: vec![OpenRouterRequestMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        self.client.chat_completions(&self.api_key, request)
    }
}

impl<C: OpenRouterClient> AiProvider for OpenRouterProvider<C> {
    fn name(&self) -> &str {
        "openrouter"
    }

    fn generate(
        &mut self,
        prompt: &str,
        _mode: DmMode,
    ) -> Result<DmResponse, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.chat_completion(prompt)?;
        let content = extract_response_content(&response)?;
        Ok(parse_dm_response(content))
    }
}

fn extract_response_content(
    response: &OpenRouterResponse,
) -> Result<&str, OpenRouterError> {
    response
        .choices
        .first()
        .and_then(|choice| choice.message.content.as_deref())
        .ok_or(OpenRouterError::EmptyResponse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::secrets::{InMemorySecureStorage, SecureStorage};
    use std::sync::{Arc, Mutex};
    use std::collections::VecDeque;

    #[derive(Clone, Default, Debug)]
    struct RecordingClient {
        state: Arc<Mutex<RecordingState>>,
    }

    #[derive(Default, Debug)]
    struct RecordingState {
        requests: Vec<OpenRouterRequest>,
        responses: VecDeque<Result<OpenRouterResponse, OpenRouterError>>,
    }

    impl RecordingClient {
        fn with_responses(
            responses: Vec<Result<OpenRouterResponse, OpenRouterError>>,
        ) -> Self {
            Self {
                state: Arc::new(Mutex::new(RecordingState {
                    requests: Vec::new(),
                    responses: responses.into(),
                })),
            }
        }
    }

    impl OpenRouterClient for RecordingClient {
        fn chat_completions(
            &self,
            _api_key: &str,
            request: OpenRouterRequest,
        ) -> Result<OpenRouterResponse, OpenRouterError> {
            let mut state = self.state.lock().unwrap();
            state.requests.push(request);
            state
                .responses
                .pop_front()
                .expect("test did not provide enough responses")
        }
    }

    fn validation_response() -> OpenRouterResponse {
        OpenRouterResponse {
            choices: vec![OpenRouterChoice {
                message: OpenRouterMessage {
                    content: Some("validation ok".to_string()),
                },
            }],
        }
    }

    fn dm_response() -> OpenRouterResponse {
        OpenRouterResponse {
            choices: vec![OpenRouterChoice {
                message: OpenRouterMessage {
                    content: Some(
                        r#"{"narration":"OpenRouter hello","tool_calls":[]}"#.to_string(),
                    ),
                },
            }],
        }
    }

    #[test]
    fn missing_model_returns_validation_error() {
        let storage = InMemorySecureStorage::new();
        storage
            .write_secret("openrouter_api_key", "sk-test-secret")
            .unwrap();
        let client = RecordingClient::with_responses(vec![Ok(validation_response())]);

        let err = OpenRouterProvider::new(None, &storage, client).unwrap_err();

        assert!(matches!(err, OpenRouterError::MissingModel(_)));
    }

    #[test]
    fn missing_api_key_returns_validation_error() {
        let storage = InMemorySecureStorage::new();
        let client = RecordingClient::with_responses(vec![Ok(validation_response())]);

        let err =
            OpenRouterProvider::new(Some("openai/gpt-4.1-mini"), &storage, client).unwrap_err();

        assert!(matches!(err, OpenRouterError::MissingApiKey(_)));
    }

    #[test]
    fn successful_validation_request_constructs_a_provider() {
        let storage = InMemorySecureStorage::new();
        storage
            .write_secret("openrouter_api_key", "sk-test-secret")
            .unwrap();
        let client = RecordingClient::with_responses(vec![Ok(validation_response())]);
        let observed = client.clone();

        let provider =
            OpenRouterProvider::new(Some("openai/gpt-4.1-mini"), &storage, client).unwrap();

        assert_eq!(provider.name(), "openrouter");
        let state = observed.state.lock().unwrap();
        assert_eq!(state.requests.len(), 1);
        assert_eq!(state.requests[0].model, "openai/gpt-4.1-mini");
        assert_eq!(state.requests[0].messages.len(), 1);
        assert_eq!(
            state.requests[0].messages[0].content,
            "Reply with a short confirmation that the model is available."
        );
    }

    #[test]
    fn openrouter_response_content_maps_into_dm_response() {
        let storage = InMemorySecureStorage::new();
        storage
            .write_secret("openrouter_api_key", "sk-test-secret")
            .unwrap();
        let client = RecordingClient::with_responses(vec![
            Ok(validation_response()),
            Ok(dm_response()),
        ]);

        let mut provider =
            OpenRouterProvider::new(Some("openai/gpt-4.1-mini"), &storage, client).unwrap();

        let response = provider.generate("say hello", DmMode::Conversation).unwrap();

        assert_eq!(response.narration, "OpenRouter hello");
        assert!(response.tool_calls.is_empty());
    }
}
