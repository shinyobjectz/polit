use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiProviderKind {
    #[serde(rename = "codex")]
    Codex,
    #[serde(rename = "openrouter")]
    OpenRouter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiSetupState {
    Required,
    Ready(AiConfig),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: AiProviderKind,
    pub model: Option<String>,
    #[serde(default, skip_serializing, skip_deserializing)]
    pub openrouter_api_key: Option<String>,
}

impl AiConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn setup_required(path: impl AsRef<Path>) -> AiSetupState {
        match Self::load(path) {
            Ok(config) => AiSetupState::Ready(config),
            Err(_) => AiSetupState::Required,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_ai_toml_reports_setup_required() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ai.toml");

        let state = AiConfig::setup_required(&path);

        assert_eq!(state, AiSetupState::Required);
    }

    #[test]
    fn valid_persisted_provider_metadata_loads_successfully() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ai.toml");
        let config = AiConfig {
            provider: AiProviderKind::OpenRouter,
            model: Some("openrouter/deepseek-r1".to_string()),
            openrouter_api_key: None,
        };

        config.save(&path).unwrap();

        match AiConfig::setup_required(&path) {
            AiSetupState::Ready(loaded) => {
                assert_eq!(loaded.provider, AiProviderKind::OpenRouter);
                assert_eq!(loaded.model.as_deref(), Some("openrouter/deepseek-r1"));
            }
            other => panic!("expected Ready state, got {other:?}"),
        }
    }

    #[test]
    fn config_serialization_excludes_secret_fields() {
        let config = AiConfig {
            provider: AiProviderKind::Codex,
            model: Some("codex".to_string()),
            openrouter_api_key: Some("sk-test-secret".to_string()),
        };

        let toml = toml::to_string(&config).unwrap();

        assert!(!toml.contains("sk-test-secret"));
        assert!(!toml.contains("openrouter_api_key"));
    }

    #[test]
    fn plaintext_openrouter_key_is_ignored_on_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ai.toml");
        let toml = r#"
provider = "openrouter"
model = "openrouter/deepseek-r1"
openrouter_api_key = "sk-plaintext-secret"
"#;

        std::fs::write(&path, toml).unwrap();

        let loaded = AiConfig::load(&path).unwrap();

        assert_eq!(loaded.provider, AiProviderKind::OpenRouter);
        assert_eq!(loaded.model.as_deref(), Some("openrouter/deepseek-r1"));
        assert_eq!(loaded.openrouter_api_key, None);
    }
}
