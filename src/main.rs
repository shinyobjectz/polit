mod engine;
mod systems;
mod ai;
mod ui;
mod persistence;
mod scripting;

use engine::paths::GamePaths;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--headless".to_string()) {
        tracing_subscriber::fmt::init();
        tracing::info!("Starting POLIT in headless mode");
        engine::run_headless()?;
    } else if args.contains(&"--demo".to_string()) {
        init_file_logger();
        ui::run_demo()?;
    } else {
        init_file_logger();
        ui::run_app()?;
    }

    Ok(())
}

fn init_file_logger() {
    if let Ok(paths) = GamePaths::init() {
        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&paths.log)
        {
            tracing_subscriber::fmt()
                .with_writer(file)
                .with_ansi(false)
                .init();
            return;
        }
    }
    tracing_subscriber::fmt()
        .with_writer(std::io::sink)
        .init();
}
