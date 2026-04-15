mod ai;
mod devtools;
mod engine;
mod mcp;
mod scripting;
mod state;
mod systems;
mod ui;

use engine::paths::GamePaths;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--headless".to_string()) {
        tracing_subscriber::fmt::init();
        tracing::info!("Starting POLIT in headless mode");
        engine::run_headless()?;
    } else if args.contains(&"--query".to_string()) {
        tracing_subscriber::fmt::init();
        run_query(&args)?;
    } else if args.contains(&"--mock".to_string()) {
        // Explicit mock mode (for testing without model)
        init_file_logger();
        ai::debug_log::DebugLog::init();
        ui::run_mock_app()?;
    } else {
        // Default: always use configured AI provider
        init_file_logger();
        ai::debug_log::DebugLog::init();
        ui::run_app()?;
    }

    Ok(())
}

fn run_query(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let prompt = args
        .iter()
        .position(|a| a == "--query")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("Hello");

    let paths = GamePaths::init()?;
    let config_path = paths.config.join("ai.toml");
    ensure_ai_setup_for_query(&config_path)?;
    let factory = ai::factory::ConfiguredAiProviderFactory::new(config_path);
    let mut provider = factory
        .build_provider_for_runtime()
        .map_err(|e| -> Box<dyn std::error::Error> { format!("{}", e).into() })?;

    tracing::info!("Generating...");
    let dm_prompt = ai::native_format::build_prompt(
        "You are the dungeon master for POLIT, an American politics simulator. Respond in character.",
        &ai::native_format::tool_declarations(ai::DmMode::DungeonMaster),
        &[],
        prompt,
    );

    use ai::AiProvider;
    match provider.generate(&dm_prompt, ai::DmMode::DungeonMaster) {
        Ok(response) => {
            println!("\n{}\n", response.narration);
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    Ok(())
}

fn ensure_ai_setup_for_query(
    config_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if !ui::setup::should_open_setup(config_path) {
        return Ok(());
    }

    let mut terminal = ratatui::init();
    let mut events = crate::devtools::harness::CrosstermEventSource;
    let result = ui::setup::run_setup_flow(
        &mut terminal,
        &mut events,
        config_path.to_path_buf(),
        true,
        Some("AI setup is required before using --query.".to_string()),
    );
    ratatui::restore();

    match result? {
        ui::setup::SetupOutcome::Configured => Ok(()),
        ui::setup::SetupOutcome::Cancelled => Err("AI setup was cancelled".into()),
    }
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
