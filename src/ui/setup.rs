use std::path::{Path, PathBuf};

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::backend::Backend;
use ratatui::prelude::*;
use ratatui::Terminal;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::ai::config::{AiConfig, AiProviderKind, AiSetupState};
use crate::ai::codex::CodexProvider;
use crate::ai::openrouter::OpenRouterProvider;
use crate::ai::secrets::{
    load_openrouter_api_key, save_openrouter_api_key, InMemorySecureStorage,
    KeyringSecureStorage, SecureStorage, SecureStorageError,
};
use crate::devtools::harness::EventSource;

use super::theme;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupError {
    MissingModel(String),
    MissingApiKey(String),
    SecureStorageUnavailable(String),
    ValidationFailed(String),
    PersistFailed(String),
}

impl std::fmt::Display for SetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetupError::MissingModel(message)
            | SetupError::MissingApiKey(message)
            | SetupError::SecureStorageUnavailable(message)
            | SetupError::ValidationFailed(message)
            | SetupError::PersistFailed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for SetupError {}

pub trait SetupValidator {
    fn validate_codex(&self) -> Result<(), String>;
    fn validate_openrouter(&self, model: &str, api_key: &str) -> Result<(), String>;
}

#[derive(Debug, Default)]
pub struct RealSetupValidator;

impl SetupValidator for RealSetupValidator {
    fn validate_codex(&self) -> Result<(), String> {
        CodexProvider::system()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn validate_openrouter(&self, model: &str, api_key: &str) -> Result<(), String> {
        let storage = InMemorySecureStorage::new();
        save_openrouter_api_key(&storage, api_key).map_err(|error| error.to_string())?;
        OpenRouterProvider::system(Some(model), &storage)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

pub fn should_open_setup(path: impl AsRef<Path>) -> bool {
    should_open_setup_with(
        path,
        &KeyringSecureStorage::default(),
        &RealSetupValidator,
    )
}

fn should_open_setup_with(
    path: impl AsRef<Path>,
    storage: &dyn SecureStorage,
    validator: &dyn SetupValidator,
) -> bool {
    let config = match AiConfig::setup_required(path) {
        AiSetupState::Required => return true,
        AiSetupState::Ready(config) => config,
    };

    match config.provider {
        AiProviderKind::Codex => validator.validate_codex().is_err(),
        AiProviderKind::OpenRouter => {
            let Some(model) = config.model.as_deref().map(str::trim).filter(|value| !value.is_empty()) else {
                return true;
            };
            let Some(api_key) = load_openrouter_api_key(storage)
                .ok()
                .flatten()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
            else {
                return true;
            };

            validator.validate_openrouter(model, &api_key).is_err()
        }
    }
}

pub fn persist_codex_setup(
    path: impl AsRef<Path>,
    validator: &dyn SetupValidator,
) -> Result<(), SetupError> {
    validator
        .validate_codex()
        .map_err(SetupError::ValidationFailed)?;

    AiConfig {
        provider: AiProviderKind::Codex,
        model: None,
        openrouter_api_key: None,
    }
    .save(path)
    .map_err(|error| SetupError::PersistFailed(error.to_string()))
}

pub fn persist_openrouter_setup(
    path: impl AsRef<Path>,
    storage: &dyn SecureStorage,
    validator: &dyn SetupValidator,
    model: &str,
    api_key: &str,
) -> Result<(), SetupError> {
    let model = model.trim();
    let api_key = api_key.trim();

    if model.is_empty() {
        return Err(SetupError::MissingModel(
            "openrouter model is required".to_string(),
        ));
    }

    if api_key.is_empty() {
        return Err(SetupError::MissingApiKey(
            "openrouter api key is required".to_string(),
        ));
    }

    validator
        .validate_openrouter(model, api_key)
        .map_err(SetupError::ValidationFailed)?;

    save_openrouter_api_key(storage, api_key).map_err(map_storage_error)?;

    AiConfig {
        provider: AiProviderKind::OpenRouter,
        model: Some(model.to_string()),
        openrouter_api_key: None,
    }
    .save(path)
    .map_err(|error| SetupError::PersistFailed(error.to_string()))
}

fn map_storage_error(error: SecureStorageError) -> SetupError {
    match error {
        SecureStorageError::Unavailable(message) => SetupError::SecureStorageUnavailable(format!(
            "secure storage is unavailable, so POLIT cannot save the OpenRouter key: {}",
            message
        )),
        SecureStorageError::Failed(message) => SetupError::PersistFailed(format!(
            "failed to save the OpenRouter key securely: {}",
            message
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupOutcome {
    Configured,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SetupField {
    Provider,
    Model,
    ApiKey,
    Save,
}

pub struct SetupScreen<V = RealSetupValidator, S = KeyringSecureStorage> {
    config_path: PathBuf,
    validator: V,
    storage: S,
    required: bool,
    provider: AiProviderKind,
    model: String,
    api_key: String,
    active_field: SetupField,
    status: Option<String>,
}

impl SetupScreen<RealSetupValidator, KeyringSecureStorage> {
    pub fn from_existing(
        config_path: impl Into<PathBuf>,
        required: bool,
        initial_status: Option<String>,
    ) -> Self {
        Self::new(
            config_path,
            RealSetupValidator,
            KeyringSecureStorage::default(),
            required,
            initial_status,
        )
    }
}

impl<V: SetupValidator, S: SecureStorage> SetupScreen<V, S> {
    pub fn new(
        config_path: impl Into<PathBuf>,
        validator: V,
        storage: S,
        required: bool,
        initial_status: Option<String>,
    ) -> Self {
        let config_path = config_path.into();
        let existing = AiConfig::load(&config_path).ok();
        let stored_api_key = load_openrouter_api_key(&storage)
            .ok()
            .flatten()
            .unwrap_or_default();

        Self {
            config_path,
            validator,
            storage,
            required,
            provider: existing
                .as_ref()
                .map(|config| config.provider)
                .unwrap_or(AiProviderKind::Codex),
            model: existing
                .as_ref()
                .and_then(|config| config.model.clone())
                .unwrap_or_else(|| "openrouter/deepseek-r1".to_string()),
            api_key: stored_api_key,
            active_field: SetupField::Provider,
            status: initial_status,
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut Terminal<impl Backend>,
        events: &mut impl EventSource,
    ) -> Result<SetupOutcome, Box<dyn std::error::Error>> {
        loop {
            terminal.draw(|frame| self.render(frame))?;

            if !events.poll(std::time::Duration::from_millis(50))? {
                continue;
            }

            if let Event::Key(key) = events.read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Char('c')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        return Ok(SetupOutcome::Cancelled);
                    }
                    KeyCode::Esc | KeyCode::Char('q') if !self.required => {
                        return Ok(SetupOutcome::Cancelled);
                    }
                    KeyCode::Up => self.move_previous_field(),
                    KeyCode::Down | KeyCode::Tab => self.move_next_field(),
                    KeyCode::Left => self.adjust_provider(false),
                    KeyCode::Right => self.adjust_provider(true),
                    KeyCode::Enter => {
                        if self.active_field == SetupField::Save {
                            match self.persist_selection() {
                                Ok(()) => return Ok(SetupOutcome::Configured),
                                Err(error) => self.status = Some(error.to_string()),
                            }
                        } else if self.active_field == SetupField::Provider
                            && self.provider == AiProviderKind::Codex
                        {
                            match self.persist_selection() {
                                Ok(()) => return Ok(SetupOutcome::Configured),
                                Err(error) => self.status = Some(error.to_string()),
                            }
                        } else {
                            self.move_next_field();
                        }
                    }
                    KeyCode::Backspace => {
                        if self.active_field == SetupField::Model {
                            self.model.pop();
                        } else if self.active_field == SetupField::ApiKey {
                            self.api_key.pop();
                        }
                    }
                    KeyCode::Char(c) => {
                        if self.active_field == SetupField::Model {
                            self.model.push(c);
                        } else if self.active_field == SetupField::ApiKey {
                            self.api_key.push(c);
                        } else if self.active_field == SetupField::Provider {
                            match c {
                                'c' | 'C' => self.provider = AiProviderKind::Codex,
                                'o' | 'O' => self.provider = AiProviderKind::OpenRouter,
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn persist_selection(&self) -> Result<(), SetupError> {
        match self.provider {
            AiProviderKind::Codex => persist_codex_setup(&self.config_path, &self.validator),
            AiProviderKind::OpenRouter => persist_openrouter_setup(
                &self.config_path,
                &self.storage,
                &self.validator,
                &self.model,
                &self.api_key,
            ),
        }
    }

    fn move_previous_field(&mut self) {
        self.active_field = match self.provider {
            AiProviderKind::Codex => match self.active_field {
                SetupField::Provider => SetupField::Save,
                SetupField::Save | SetupField::Model | SetupField::ApiKey => SetupField::Provider,
            },
            AiProviderKind::OpenRouter => match self.active_field {
                SetupField::Provider => SetupField::Save,
                SetupField::Model => SetupField::Provider,
                SetupField::ApiKey => SetupField::Model,
                SetupField::Save => SetupField::ApiKey,
            },
        };
    }

    fn move_next_field(&mut self) {
        self.active_field = match self.provider {
            AiProviderKind::Codex => match self.active_field {
                SetupField::Provider | SetupField::Model | SetupField::ApiKey => SetupField::Save,
                SetupField::Save => SetupField::Provider,
            },
            AiProviderKind::OpenRouter => match self.active_field {
                SetupField::Provider => SetupField::Model,
                SetupField::Model => SetupField::ApiKey,
                SetupField::ApiKey => SetupField::Save,
                SetupField::Save => SetupField::Provider,
            },
        };
    }

    fn adjust_provider(&mut self, advance: bool) {
        if self.active_field != SetupField::Provider {
            return;
        }

        self.provider = match (self.provider, advance) {
            (AiProviderKind::Codex, true) => AiProviderKind::OpenRouter,
            (AiProviderKind::OpenRouter, true) => AiProviderKind::Codex,
            (AiProviderKind::Codex, false) => AiProviderKind::OpenRouter,
            (AiProviderKind::OpenRouter, false) => AiProviderKind::Codex,
        };

        if self.provider == AiProviderKind::Codex && self.active_field == SetupField::ApiKey {
            self.active_field = SetupField::Provider;
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        frame.render_widget(Block::default().style(Style::default().bg(theme::BG)), area);

        let popup = centered_rect(78, 24, area);
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Block::default()
                .title(" AI Setup ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BORDER_ACTIVE))
                .style(Style::default().bg(theme::BG_SUBTLE)),
            popup,
        );

        let inner = Rect::new(
            popup.x + 2,
            popup.y + 1,
            popup.width.saturating_sub(4),
            popup.height.saturating_sub(2),
        );

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(2),
                Constraint::Length(2),
            ])
            .split(inner);

        let heading = Paragraph::new("Choose how POLIT should handle all AI interactions.")
            .style(Style::default().fg(theme::FG));
        frame.render_widget(heading, chunks[0]);

        let subheading = if self.required {
            "Setup is required before gameplay can continue."
        } else {
            "Update the provider or model, then validate and save."
        };
        frame.render_widget(
            Paragraph::new(subheading).style(Style::default().fg(theme::FG_DIM)),
            chunks[1],
        );

        frame.render_widget(self.render_provider_line(), chunks[2]);
        frame.render_widget(self.render_model_line(), chunks[3]);
        frame.render_widget(self.render_api_key_line(), chunks[4]);
        frame.render_widget(self.render_save_line(), chunks[5]);

        let help = match self.provider {
            AiProviderKind::Codex => {
                "Codex requires a local `codex` install and an authenticated session."
            }
            AiProviderKind::OpenRouter => {
                "OpenRouter validates the model live and stores the API key only in secure storage."
            }
        };
        frame.render_widget(
            Paragraph::new(help).style(Style::default().fg(theme::FG_DIM)),
            chunks[6],
        );

        let footer = if self.required {
            "Left/Right provider  Up/Down move  Enter validate  Ctrl+C quit"
        } else {
            "Left/Right provider  Up/Down move  Enter validate  Q back"
        };
        frame.render_widget(
            Paragraph::new(footer).alignment(Alignment::Center).style(
                Style::default().fg(theme::FG_MUTED),
            ),
            chunks[7],
        );

        if let Some(message) = &self.status {
            let status_area = Rect::new(
                popup.x + 2,
                popup.y + popup.height.saturating_sub(4),
                popup.width.saturating_sub(4),
                2,
            );
            frame.render_widget(
                Paragraph::new(message.as_str())
                    .wrap(ratatui::widgets::Wrap { trim: true })
                    .style(Style::default().fg(theme::WARNING)),
                status_area,
            );
        }
    }

    fn render_provider_line(&self) -> Paragraph<'static> {
        let codex_style = if self.provider == AiProviderKind::Codex {
            Style::default().fg(theme::ACCENT).bold()
        } else {
            Style::default().fg(theme::FG_DIM)
        };
        let openrouter_style = if self.provider == AiProviderKind::OpenRouter {
            Style::default().fg(theme::ACCENT).bold()
        } else {
            Style::default().fg(theme::FG_DIM)
        };

        Paragraph::new(Line::from(vec![
            self.field_marker(SetupField::Provider),
            Span::styled("Provider: ", self.field_label_style(SetupField::Provider)),
            Span::styled("Codex Subscription", codex_style),
            Span::raw("   "),
            Span::styled("OpenRouter", openrouter_style),
        ]))
    }

    fn render_model_line(&self) -> Paragraph<'_> {
        let content = if self.provider == AiProviderKind::Codex {
            "Not needed for Codex.".to_string()
        } else {
            self.model.clone()
        };

        Paragraph::new(Line::from(vec![
            self.field_marker(SetupField::Model),
            Span::styled("OpenRouter Model: ", self.field_label_style(SetupField::Model)),
            Span::styled(content, self.field_value_style(SetupField::Model)),
        ]))
    }

    fn render_api_key_line(&self) -> Paragraph<'_> {
        let content = if self.provider == AiProviderKind::Codex {
            "Not needed for Codex.".to_string()
        } else if self.api_key.is_empty() {
            "Enter API key".to_string()
        } else {
            "*".repeat(self.api_key.chars().count().max(8))
        };

        Paragraph::new(Line::from(vec![
            self.field_marker(SetupField::ApiKey),
            Span::styled("OpenRouter Key: ", self.field_label_style(SetupField::ApiKey)),
            Span::styled(content, self.field_value_style(SetupField::ApiKey)),
        ]))
    }

    fn render_save_line(&self) -> Paragraph<'static> {
        let label = match self.provider {
            AiProviderKind::Codex => "Validate Codex and save",
            AiProviderKind::OpenRouter => "Validate OpenRouter and save",
        };

        Paragraph::new(Line::from(vec![
            self.field_marker(SetupField::Save),
            Span::styled(label, self.field_label_style(SetupField::Save)),
        ]))
    }

    fn field_marker(&self, field: SetupField) -> Span<'static> {
        if self.active_field == field {
            Span::styled("▶ ", Style::default().fg(theme::ACCENT).bold())
        } else {
            Span::styled("  ", Style::default())
        }
    }

    fn field_label_style(&self, field: SetupField) -> Style {
        if self.active_field == field {
            Style::default().fg(theme::FG).bold()
        } else {
            Style::default().fg(theme::FG)
        }
    }

    fn field_value_style(&self, field: SetupField) -> Style {
        if self.provider == AiProviderKind::Codex
            && matches!(field, SetupField::Model | SetupField::ApiKey)
        {
            Style::default().fg(theme::FG_MUTED)
        } else if self.active_field == field {
            Style::default().fg(theme::ACCENT_BLUE)
        } else {
            Style::default().fg(theme::FG_DIM)
        }
    }
}

pub fn run_setup_flow(
    terminal: &mut Terminal<impl Backend>,
    events: &mut impl EventSource,
    config_path: impl Into<PathBuf>,
    required: bool,
    initial_status: Option<String>,
) -> Result<SetupOutcome, Box<dyn std::error::Error>> {
    let mut screen = SetupScreen::from_existing(config_path, required, initial_status);
    screen.run(terminal, events)
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::secrets::{InMemorySecureStorage, SecureStorage};
    use std::cell::Cell;
    use std::fs;
    use tempfile::tempdir;

    struct FakeValidator {
        codex_result: Result<(), String>,
        openrouter_result: Result<(), String>,
        codex_calls: Cell<u32>,
        openrouter_calls: Cell<u32>,
    }

    impl FakeValidator {
        fn passing() -> Self {
            Self {
                codex_result: Ok(()),
                openrouter_result: Ok(()),
                codex_calls: Cell::new(0),
                openrouter_calls: Cell::new(0),
            }
        }

        fn failing(message: &str) -> Self {
            Self {
                codex_result: Err(message.to_string()),
                openrouter_result: Err(message.to_string()),
                codex_calls: Cell::new(0),
                openrouter_calls: Cell::new(0),
            }
        }
    }

    impl SetupValidator for FakeValidator {
        fn validate_codex(&self) -> Result<(), String> {
            self.codex_calls.set(self.codex_calls.get() + 1);
            self.codex_result.clone()
        }

        fn validate_openrouter(&self, _model: &str, _api_key: &str) -> Result<(), String> {
            self.openrouter_calls.set(self.openrouter_calls.get() + 1);
            self.openrouter_result.clone()
        }
    }

    #[test]
    fn missing_or_invalid_config_routes_into_setup() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ai.toml");

        assert!(should_open_setup_with(
            &path,
            &InMemorySecureStorage::new(),
            &FakeValidator::passing(),
        ));

        fs::write(&path, "provider = [").unwrap();
        assert!(should_open_setup_with(
            &path,
            &InMemorySecureStorage::new(),
            &FakeValidator::passing(),
        ));
    }

    #[test]
    fn missing_openrouter_secret_routes_back_into_setup() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ai.toml");
        let config = AiConfig {
            provider: AiProviderKind::OpenRouter,
            model: Some("openrouter/deepseek-r1".to_string()),
            openrouter_api_key: None,
        };
        config.save(&path).unwrap();

        assert!(should_open_setup_with(
            &path,
            &InMemorySecureStorage::new(),
            &FakeValidator::passing(),
        ));
    }

    #[test]
    fn invalid_provider_state_routes_back_into_setup() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ai.toml");
        let config = AiConfig {
            provider: AiProviderKind::Codex,
            model: None,
            openrouter_api_key: None,
        };
        config.save(&path).unwrap();

        assert!(should_open_setup_with(
            &path,
            &InMemorySecureStorage::new(),
            &FakeValidator::failing("codex unavailable"),
        ));
    }

    #[test]
    fn selecting_codex_runs_codex_validation_and_persists_provider_metadata_on_success() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ai.toml");
        let validator = FakeValidator::passing();

        persist_codex_setup(&path, &validator).unwrap();

        let config = AiConfig::load(&path).unwrap();
        assert_eq!(config.provider, AiProviderKind::Codex);
        assert_eq!(config.model, None);
        assert_eq!(validator.codex_calls.get(), 1);
        assert_eq!(validator.openrouter_calls.get(), 0);
    }

    #[test]
    fn selecting_openrouter_requires_model_plus_secure_key_save_and_validation() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ai.toml");
        let storage = InMemorySecureStorage::new();
        let validator = FakeValidator::passing();

        assert!(matches!(
            persist_openrouter_setup(&path, &storage, &validator, "", "sk-test"),
            Err(SetupError::MissingModel(_))
        ));

        assert!(matches!(
            persist_openrouter_setup(
                &path,
                &storage,
                &validator,
                "openrouter/deepseek-r1",
                "",
            ),
            Err(SetupError::MissingApiKey(_))
        ));

        persist_openrouter_setup(
            &path,
            &storage,
            &validator,
            "openrouter/deepseek-r1",
            "sk-test",
        )
        .unwrap();

        let config = AiConfig::load(&path).unwrap();
        assert_eq!(config.provider, AiProviderKind::OpenRouter);
        assert_eq!(config.model.as_deref(), Some("openrouter/deepseek-r1"));
        assert_eq!(config.openrouter_api_key, None);
        assert_eq!(validator.codex_calls.get(), 0);
        assert_eq!(validator.openrouter_calls.get(), 1);
        assert_eq!(
            storage.read_secret("openrouter_api_key").unwrap().as_deref(),
            Some("sk-test")
        );
    }

    #[test]
    fn validation_failure_keeps_the_user_in_setup() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ai.toml");
        let storage = InMemorySecureStorage::new();
        let validator = FakeValidator::failing("validation failed");

        let err = persist_codex_setup(&path, &validator).unwrap_err();

        assert!(matches!(err, SetupError::ValidationFailed(_)));
        assert!(!path.exists());
        assert!(storage.read_secret("openrouter_api_key").unwrap().is_none());
    }

    #[test]
    fn existing_openrouter_setup_prefills_the_stored_api_key() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ai.toml");
        let storage = InMemorySecureStorage::new();

        AiConfig {
            provider: AiProviderKind::OpenRouter,
            model: Some("openrouter/deepseek-r1".to_string()),
            openrouter_api_key: None,
        }
        .save(&path)
        .unwrap();
        save_openrouter_api_key(&storage, "sk-existing-secret").unwrap();

        let screen = SetupScreen::new(path, FakeValidator::passing(), storage, false, None);

        assert_eq!(screen.provider, AiProviderKind::OpenRouter);
        assert_eq!(screen.model, "openrouter/deepseek-r1");
        assert_eq!(screen.api_key, "sk-existing-secret");
    }
}
