pub mod app;
pub mod chat;
pub mod title;

use crate::engine::channels::Channels;
use crate::engine::paths::GamePaths;
use crate::engine::GameState;
use crate::engine::{demo, game_thread};

use title::{TitleAction, TitleScreen};

/// Launch the full game with title screen
pub fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    let paths = GamePaths::init()?;
    let has_save = paths
        .saves
        .read_dir()
        .map(|mut d| d.next().is_some())
        .unwrap_or(false);

    // Enter alternate screen ONCE for the entire session
    let mut terminal = ratatui::init();

    // Show title screen
    let mut title = TitleScreen::new(has_save);
    let action = title.run(&mut terminal)?;

    match action {
        TitleAction::Quit => {
            ratatui::restore();
            return Ok(());
        }
        TitleAction::Settings => {
            ratatui::restore();
            return Ok(());
        }
        TitleAction::Demo => {
            // Run demo in the SAME terminal session (no restore/reinit)
            let state = GameState::new(paths.db.to_str().unwrap())?;
            let channels = Channels::new();
            let (ui_channels, game_channels) = channels.split();

            let game_handle = std::thread::Builder::new()
                .name("polit-demo".to_string())
                .spawn(move || {
                    demo::run_demo(state, game_channels);
                })
                .expect("Failed to spawn demo thread");

            let mut app_inst = app::App::new(ui_channels);
            let result = app_inst.run(&mut terminal);
            ratatui::restore();
            let _ = game_handle.join();
            return result;
        }
        TitleAction::NewCampaign | TitleAction::ContinueCampaign => {
            // Continue in SAME terminal session
            let state = GameState::new(paths.db.to_str().unwrap())?;
            let channels = Channels::new();
            let (ui_channels, game_channels) = channels.split();
            let game_handle = game_thread::spawn_game_thread(state, game_channels);

            let mut app_inst = app::App::new(ui_channels);
            let result = app_inst.run(&mut terminal);
            ratatui::restore();
            let _ = game_handle.join();
            return result;
        }
    }
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

    let mut terminal = ratatui::init();
    let mut app_inst = app::App::new(ui_channels);
    let result = app_inst.run(&mut terminal);
    ratatui::restore();

    let _ = game_handle.join();
    result
}
