pub mod app;
pub mod chat;

use crate::engine::channels::Channels;
use crate::engine::paths::GamePaths;
use crate::engine::{demo, game_thread};
use crate::engine::GameState;

/// Launch the TUI application with separate game thread
pub fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    let (state, ui_channels, game_channels) = setup()?;
    let game_handle = game_thread::spawn_game_thread(state, game_channels);
    run_ui(ui_channels)?;
    let _ = game_handle.join();
    Ok(())
}

/// Launch the TUI with a scripted demo walkthrough
pub fn run_demo() -> Result<(), Box<dyn std::error::Error>> {
    let (state, ui_channels, game_channels) = setup()?;
    let game_handle = std::thread::Builder::new()
        .name("polit-demo".to_string())
        .spawn(move || {
            demo::run_demo(state, game_channels);
        })
        .expect("Failed to spawn demo thread");
    run_ui(ui_channels)?;
    let _ = game_handle.join();
    Ok(())
}

fn setup() -> Result<(GameState, crate::engine::channels::UiChannels, crate::engine::channels::GameChannels), Box<dyn std::error::Error>> {
    let paths = GamePaths::init()?;
    let state = GameState::new(paths.db.to_str().unwrap())?;
    let channels = Channels::new();
    let (ui_channels, game_channels) = channels.split();
    Ok((state, ui_channels, game_channels))
}

fn run_ui(ui_channels: crate::engine::channels::UiChannels) -> Result<(), Box<dyn std::error::Error>> {
    // ratatui::init() enters alternate screen buffer automatically —
    // game runs in its own screen, terminal restored perfectly on exit
    let mut terminal = ratatui::init();
    let mut app_inst = app::App::new(ui_channels);
    let result = app_inst.run(&mut terminal);
    ratatui::restore();
    result
}
