mod ai;
mod engine;
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
        ui::run_app()?;
    } else {
        // Default: always use real model
        init_file_logger();
        ai::debug_log::DebugLog::init();
        let model_id = get_model_id(&args);
        let hf_token = std::env::var("HF_TOKEN").ok();

        eprintln!("Loading Gemma 4 model...");

        // Suppress llama.cpp Metal shader compilation noise on stderr
        let provider = {
            use std::os::unix::io::AsRawFd;
            let devnull = std::fs::File::open("/dev/null").ok();
            let saved_stderr = unsafe { libc::dup(2) };
            if let Some(ref null) = devnull {
                unsafe {
                    libc::dup2(null.as_raw_fd(), 2);
                }
            }
            let result = ai::provider::CandleProvider::load(model_id, hf_token.as_deref());
            // Restore stderr
            if saved_stderr >= 0 {
                unsafe {
                    libc::dup2(saved_stderr, 2);
                    libc::close(saved_stderr);
                }
            }
            result
        }
        .map_err(|e| -> Box<dyn std::error::Error> { format!("{}", e).into() })?;

        eprintln!("Ready.");

        ui::run_app_with_provider(Box::new(provider))?;
    }

    Ok(())
}

fn run_query(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let model_id = get_model_id(args);
    let hf_token = std::env::var("HF_TOKEN").ok();

    let prompt = args
        .iter()
        .position(|a| a == "--query")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("Hello");

    tracing::info!("Loading model: {}", model_id);
    let mut provider = ai::provider::CandleProvider::load(model_id, hf_token.as_deref())
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

fn get_model_id<'a>(args: &'a [String]) -> &'a str {
    args.iter()
        .position(|a| a == "--model")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("google/gemma-4-E4B-it")
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
