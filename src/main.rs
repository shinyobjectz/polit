mod engine;
mod systems;
mod ai;
mod ui;
mod persistence;
mod scripting;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--headless".to_string()) {
        // Headless mode: log to stderr
        tracing_subscriber::fmt::init();
        tracing::info!("Starting POLIT in headless mode");
        engine::run_headless()?;
    } else if args.contains(&"--demo".to_string()) {
        // Demo/TUI modes: log to file so terminal stays clean
        init_file_logger();
        ui::run_demo()?;
    } else {
        init_file_logger();
        ui::run_app()?;
    }

    Ok(())
}

fn init_file_logger() {
    use tracing_subscriber::fmt::writer::MakeWriterExt;

    if let Ok(log_dir) = std::env::var("HOME").map(|h| format!("{}/.polit", h)) {
        let _ = std::fs::create_dir_all(&log_dir);
        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(format!("{}/polit.log", log_dir))
        {
            tracing_subscriber::fmt()
                .with_writer(file)
                .with_ansi(false)
                .init();
            return;
        }
    }
    // Fallback: discard logs in TUI mode rather than polluting terminal
    tracing_subscriber::fmt()
        .with_writer(std::io::sink)
        .init();
}
