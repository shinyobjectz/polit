pub mod app;
pub mod chat;

use crate::engine::channels::Channels;
use crate::engine::{demo, game_thread};
use crate::engine::GameState;

/// Launch the TUI application with separate game thread
pub fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    let (state, ui_channels, game_channels) = setup()?;

    // Spawn game thread (normal mode)
    let game_handle = game_thread::spawn_game_thread(state, game_channels);

    run_ui(ui_channels)?;

    let _ = game_handle.join();
    Ok(())
}

/// Launch the TUI with a scripted demo walkthrough
pub fn run_demo() -> Result<(), Box<dyn std::error::Error>> {
    let (state, ui_channels, game_channels) = setup()?;

    // Spawn demo on game thread
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
    let save_dir = dirs_or_default();
    std::fs::create_dir_all(&save_dir)?;
    let db_path = format!("{}/game.db", save_dir);
    let state = GameState::new(&db_path)?;

    let channels = Channels::new();
    let (ui_channels, game_channels) = channels.split();

    Ok((state, ui_channels, game_channels))
}

fn run_ui(ui_channels: crate::engine::channels::UiChannels) -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();
    let mut app_inst = app::App::new(ui_channels);
    let result = app_inst.run(&mut terminal);
    ratatui::restore();
    result
}

fn dirs_or_default() -> String {
    if let Some(home) = std::env::var_os("HOME") {
        format!("{}/.polit", home.to_string_lossy())
    } else {
        ".polit".to_string()
    }
}
