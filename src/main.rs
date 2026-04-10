mod ai;
mod engine;
mod persistence;
mod scripting;
mod systems;
mod ui;

use engine::paths::GamePaths;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--headless".to_string()) {
        tracing_subscriber::fmt::init();
        tracing::info!("Starting POLIT in headless mode");
        engine::run_headless()?;
    } else if args.contains(&"--load-model".to_string()) {
        // Load the real Gemma 4 model, then launch game
        tracing_subscriber::fmt::init();
        let model_id = args
            .iter()
            .position(|a| a == "--model")
            .and_then(|i| args.get(i + 1))
            .map(|s| s.as_str())
            .unwrap_or("google/gemma-4-E2B-it");

        let hf_token = std::env::var("HF_TOKEN").ok();

        tracing::info!("Loading model: {}", model_id);
        let provider = ai::provider::CandleProvider::load(model_id, hf_token.as_deref())
            .map_err(|e| -> Box<dyn std::error::Error> { format!("{}", e).into() })?;
        tracing::info!("Model loaded. Starting game...");

        // TODO: pass provider into game state and launch TUI
        drop(provider);
        println!("Model loaded successfully. TUI integration coming next.");
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
    tracing_subscriber::fmt().with_writer(std::io::sink).init();
}
