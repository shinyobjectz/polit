pub mod app;
pub mod character_creation;
pub mod chat;
pub mod components;
pub mod intro;
pub mod music;
pub mod scenario;
pub mod theme;
pub mod title;
pub mod setup;

use crate::engine::channels::Channels;
use crate::engine::paths::GamePaths;
use crate::engine::GameState;
use crate::engine::{demo, game_thread};
use crate::ai::factory::ConfiguredAiProviderFactory;

use music::MusicController;
use setup::{run_setup_flow, should_open_setup, SetupOutcome};
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
    let ai_config_path = paths.config.join("ai.toml");
    let factory = ConfiguredAiProviderFactory::new(ai_config_path.clone());

    let mut terminal = init_terminal();

    if should_open_setup(&ai_config_path) {
        if run_setup_flow(&mut terminal, ai_config_path.clone(), true, None)?
            == SetupOutcome::Cancelled
        {
            restore_terminal();
            return Ok(());
        }
    }

    // Music lives for the entire pre-game flow
    let music = MusicController::start_anthem();

    loop {
        let has_save = paths
            .saves
            .join("current")
            .join("character.yaml")
            .exists();

        // Show title screen
        let mut title = TitleScreen::new(has_save);
        let action = title.run(&mut terminal, &music)?;

        match action {
            TitleAction::Quit => {
                music.stop();
                restore_terminal();
                return Ok(());
            }
            TitleAction::Settings => {
                let _ = run_setup_flow(&mut terminal, ai_config_path.clone(), false, None)?;
                continue;
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

                // Character creation (configured AI)
                let mut char_screen = character_creation::CharacterCreationScreen::new();
                let character_provider = match factory.build_provider_for_character_creation() {
                    Ok(provider) => provider,
                    Err(error) => {
                        music.switch_to_anthem();
                        let _ = run_setup_flow(
                            &mut terminal,
                            ai_config_path.clone(),
                            false,
                            Some(error.to_string()),
                        )?;
                        continue;
                    }
                };
                let _character = char_screen.run(&mut terminal, character_provider, &music)?;

                let runtime_provider = match factory.build_provider_for_runtime() {
                    Ok(provider) => provider,
                    Err(error) => {
                        music.switch_to_anthem();
                        let _ = run_setup_flow(
                            &mut terminal,
                            ai_config_path.clone(),
                            false,
                            Some(error.to_string()),
                        )?;
                        continue;
                    }
                };

                // Stop music before entering the game
                music.stop();

                let state = GameState::with_provider(runtime_provider)?;
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
                let runtime_provider = match factory.build_provider_for_runtime() {
                    Ok(provider) => provider,
                    Err(error) => {
                        let _ = run_setup_flow(
                            &mut terminal,
                            ai_config_path.clone(),
                            false,
                            Some(error.to_string()),
                        )?;
                        continue;
                    }
                };

                // Stop music before entering the game
                music.stop();

                let state = GameState::with_provider(runtime_provider)?;
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
}

/// Launch the full game with mock AI (for local testing without configured providers).
pub fn run_mock_app() -> Result<(), Box<dyn std::error::Error>> {
    let paths = GamePaths::init()?;
    let ai_config_path = paths.config.join("ai.toml");

    let mut terminal = init_terminal();

    // Music lives for the entire pre-game flow
    let music = MusicController::start_anthem();

    loop {
        let has_save = paths
            .saves
            .join("current")
            .join("character.yaml")
            .exists();

        // Show title screen
        let mut title = TitleScreen::new(has_save);
        let action = title.run(&mut terminal, &music)?;

        match action {
            TitleAction::Quit => {
                music.stop();
                restore_terminal();
                return Ok(());
            }
            TitleAction::Settings => {
                let _ = run_setup_flow(&mut terminal, ai_config_path.clone(), false, None)?;
                continue;
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

                let state = GameState::new()?;
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

                let state = GameState::new()?;
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
}

/// Launch directly into demo mode (skips title screen)
pub fn run_demo() -> Result<(), Box<dyn std::error::Error>> {
    let paths = GamePaths::init()?;
    let state = GameState::new()?;
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
