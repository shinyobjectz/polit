mod engine;
mod systems;
mod ai;
mod ui;
mod persistence;
mod scripting;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--headless".to_string()) {
        tracing::info!("Starting POLIT in headless mode");
        engine::run_headless()?;
    } else if args.contains(&"--demo".to_string()) {
        tracing::info!("Starting POLIT demo");
        ui::run_demo()?;
    } else {
        tracing::info!("Starting POLIT");
        ui::run_app()?;
    }

    Ok(())
}
