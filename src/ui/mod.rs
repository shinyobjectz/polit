pub mod app;
pub mod character_creation;
pub mod chat;
pub mod components;
pub mod intro;
pub mod music;
pub mod scenario;
pub mod theme;
pub mod title;

use crate::engine::channels::Channels;
use crate::engine::paths::GamePaths;
use crate::engine::GameState;
use crate::engine::{demo, game_thread};

use music::MusicController;
use title::{TitleAction, TitleScreen};

/// Initialize terminal with mouse support
fn init_terminal() -> ratatui::DefaultTerminal {
    let terminal = ratatui::init();
    // Enable mouse capture AFTER ratatui::init() so it's not reset by EnterAlternateScreen
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture).ok();
    terminal
}

/// Restore terminal and disable mouse capture
fn restore_terminal() {
    ratatui::restore();
    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture).ok();
}

/// Launch the full game with title screen
pub fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    let paths = GamePaths::init()?;
    let has_save = paths
        .saves
        .read_dir()
        .map(|mut d| d.next().is_some())
        .unwrap_or(false);

    let mut terminal = init_terminal();

    // Music lives for the entire pre-game flow
    let music = MusicController::start_anthem();

    // Show title screen
    let mut title = TitleScreen::new(has_save);
    let action = title.run(&mut terminal, &music)?;

    match action {
        TitleAction::Quit | TitleAction::Settings => {
            music.stop();
            restore_terminal();
            return Ok(());
        }
        TitleAction::NewCampaign => {
            // Scenario select (anthem continues)
            let mut scenario_screen = scenario::ScenarioScreen::new();
            let config = scenario_screen.run(&mut terminal, &music)?;
            if config.is_none() {
                music.stop();
                restore_terminal();
                return Ok(());
            }
            let _scenario_config = config.unwrap();

            // Cinematic intro (switches to intro score)
            let intro_toml = include_str!("../../game/scenarios/modern_usa/intro.toml");
            if let Ok(mut intro_screen) = intro::IntroScreen::from_toml(intro_toml) {
                let _ = intro_screen.run(&mut terminal, &music);
            }

            // Switch to character creation score
            music.switch_to_char_creation();

            // Character creation (mock AI)
            let mock_provider: Box<dyn crate::ai::AiProvider> =
                Box::new(crate::ai::mock::MockProvider::new());
            let mut char_screen = character_creation::CharacterCreationScreen::new();
            let _character = char_screen.run(&mut terminal, mock_provider, &music)?;

            // Stop music before entering the game
            music.stop();

            let state = GameState::new(paths.db.to_str().unwrap())?;
            let channels = Channels::new();
            let (ui_channels, game_channels) = channels.split();
            let game_handle = game_thread::spawn_game_thread(state, game_channels);

            let mut app_inst = app::App::new(ui_channels);
            let result = app_inst.run(&mut terminal);
            restore_terminal();
            let _ = game_handle.join();
            return result;
        }
        TitleAction::ContinueCampaign => {
            // Stop music before entering the game
            music.stop();

            let state = GameState::new(paths.db.to_str().unwrap())?;
            let channels = Channels::new();
            let (ui_channels, game_channels) = channels.split();
            let game_handle = game_thread::spawn_game_thread(state, game_channels);

            let mut app_inst = app::App::new(ui_channels);
            let result = app_inst.run(&mut terminal);
            restore_terminal();
            let _ = game_handle.join();
            return result;
        }
    }
}

/// Launch with a specific AI provider (e.g., real Gemma 4 model)
pub fn run_app_with_provider(
    mut provider: Box<dyn crate::ai::AiProvider>,
) -> Result<(), Box<dyn std::error::Error>> {
    let paths = GamePaths::init()?;
    let has_save = paths
        .saves
        .read_dir()
        .map(|mut d| d.next().is_some())
        .unwrap_or(false);

    let mut terminal = init_terminal();

    let music = MusicController::start_anthem();

    let mut title = TitleScreen::new(has_save);
    let action = title.run(&mut terminal, &music)?;

    match action {
        TitleAction::Quit | TitleAction::Settings => {
            music.stop();
            restore_terminal();
            return Ok(());
        }
        TitleAction::NewCampaign => {
            let mut scenario_screen = scenario::ScenarioScreen::new();
            let config = scenario_screen.run(&mut terminal, &music)?;
            if config.is_none() {
                music.stop();
                restore_terminal();
                return Ok(());
            }
            let _scenario_config = config.unwrap();

            // Cinematic intro (switches to intro score)
            let intro_toml = include_str!("../../game/scenarios/modern_usa/intro.toml");
            if let Ok(mut intro_screen) = intro::IntroScreen::from_toml(intro_toml) {
                let _ = intro_screen.run(&mut terminal, &music);
            }

            // Switch to character creation score
            music.switch_to_char_creation();

            // Character creation (AI-guided)
            let mut char_screen = character_creation::CharacterCreationScreen::new();
            let _character = char_screen.run(&mut terminal, provider, &music)?;
        }
        TitleAction::ContinueCampaign => {}
    }

    // Stop music before entering the game
    music.stop();

    // Reload provider for game (model is cached, loads in ~5s)
    eprintln!("Loading model for game...");
    let hf_token = std::env::var("HF_TOKEN").ok();
    let game_provider =
        crate::ai::provider::CandleProvider::load("google/gemma-4-E4B-it", hf_token.as_deref())
            .map_err(|e| -> Box<dyn std::error::Error> { format!("{}", e).into() })?;
    eprintln!("Ready.");
    let provider: Box<dyn crate::ai::AiProvider> = Box::new(game_provider);

    let state = GameState::with_provider(paths.db.to_str().unwrap(), provider)?;
    let channels = Channels::new();
    let (ui_channels, game_channels) = channels.split();
    let game_handle = game_thread::spawn_game_thread(state, game_channels);

    let mut app_inst = app::App::new(ui_channels);
    let result = app_inst.run(&mut terminal);
    restore_terminal();
    let _ = game_handle.join();
    result
}

/// Launch directly into demo mode (skips title screen)
pub fn run_demo() -> Result<(), Box<dyn std::error::Error>> {
    let paths = GamePaths::init()?;
    let state = GameState::new(paths.db.to_str().unwrap())?;
    let channels = Channels::new();
    let (ui_channels, game_channels) = channels.split();

    let game_handle = std::thread::Builder::new()
        .name("polit-demo".to_string())
        .spawn(move || {
            demo::run_demo(state, game_channels);
        })
        .expect("Failed to spawn demo thread");

    let mut terminal = init_terminal();
    let mut app_inst = app::App::new(ui_channels);
    let result = app_inst.run(&mut terminal);
    restore_terminal();

    let _ = game_handle.join();
    result
}
