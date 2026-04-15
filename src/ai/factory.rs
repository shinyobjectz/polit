use std::error::Error;
use std::path::PathBuf;

use super::config::{AiConfig, AiProviderKind, AiSetupState};
use super::codex::CodexProvider;
use super::openrouter::OpenRouterProvider;
use super::secrets::{KeyringSecureStorage, SecureStorage};
use super::AiProvider;

pub trait ConfiguredAiProviderBuilder: Send + Sync {
    fn build_codex_provider(
        &self,
        config: &AiConfig,
    ) -> Result<Box<dyn AiProvider>, Box<dyn Error + Send + Sync>>;

    fn build_openrouter_provider(
        &self,
        config: &AiConfig,
        storage: &dyn SecureStorage,
    ) -> Result<Box<dyn AiProvider>, Box<dyn Error + Send + Sync>>;
}

#[derive(Debug, Default)]
pub struct RealConfiguredAiProviderBuilder;

impl ConfiguredAiProviderBuilder for RealConfiguredAiProviderBuilder {
    fn build_codex_provider(
        &self,
        _config: &AiConfig,
    ) -> Result<Box<dyn AiProvider>, Box<dyn Error + Send + Sync>> {
        Ok(Box::new(CodexProvider::system()?))
    }

    fn build_openrouter_provider(
        &self,
        config: &AiConfig,
        storage: &dyn SecureStorage,
    ) -> Result<Box<dyn AiProvider>, Box<dyn Error + Send + Sync>> {
        Ok(Box::new(OpenRouterProvider::system(config.model.as_deref(), storage)?))
    }
}

pub struct ConfiguredAiProviderFactory {
    config_path: PathBuf,
    storage: Box<dyn SecureStorage>,
    builder: Box<dyn ConfiguredAiProviderBuilder>,
}

impl ConfiguredAiProviderFactory {
    pub fn new(config_path: impl Into<PathBuf>) -> Self {
        Self {
            config_path: config_path.into(),
            storage: Box::new(KeyringSecureStorage::default()),
            builder: Box::new(RealConfiguredAiProviderBuilder::default()),
        }
    }

    pub fn with_parts(
        config_path: impl Into<PathBuf>,
        storage: Box<dyn SecureStorage>,
        builder: Box<dyn ConfiguredAiProviderBuilder>,
    ) -> Self {
        Self {
            config_path: config_path.into(),
            storage,
            builder,
        }
    }

    pub fn build_provider_for_runtime(
        &self,
    ) -> Result<Box<dyn AiProvider>, Box<dyn Error + Send + Sync>> {
        self.build_provider()
    }

    pub fn build_provider_for_character_creation(
        &self,
    ) -> Result<Box<dyn AiProvider>, Box<dyn Error + Send + Sync>> {
        self.build_provider()
    }

    fn build_provider(&self) -> Result<Box<dyn AiProvider>, Box<dyn Error + Send + Sync>> {
        let config = self.load_config()?;

        match config.provider {
            AiProviderKind::Codex => self.builder.build_codex_provider(&config),
            AiProviderKind::OpenRouter => self
                .builder
                .build_openrouter_provider(&config, self.storage.as_ref()),
        }
    }

    fn load_config(&self) -> Result<AiConfig, Box<dyn Error + Send + Sync>> {
        match AiConfig::setup_required(&self.config_path) {
            AiSetupState::Ready(config) => Ok(config),
            AiSetupState::Required => Err(format!(
                "AI config is missing or invalid: {}",
                self.config_path.display()
            )
            .into()),
        }
    }
}

pub fn build_provider_for_runtime(
    factory: &ConfiguredAiProviderFactory,
) -> Result<Box<dyn AiProvider>, Box<dyn Error + Send + Sync>> {
    factory.build_provider_for_runtime()
}

pub fn build_provider_for_character_creation(
    factory: &ConfiguredAiProviderFactory,
) -> Result<Box<dyn AiProvider>, Box<dyn Error + Send + Sync>> {
    factory.build_provider_for_character_creation()
}
