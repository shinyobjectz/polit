use polit::devtools::in_process::InProcessRunner;
use polit::devtools::scenario::Scenario;

fn scenario_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("tui")
        .join("scenarios")
}

#[test]
fn checked_in_startup_scenarios_run_in_process() {
    let runner = InProcessRunner::new();

    for name in [
        "first_launch_ai_setup_codex.yaml",
        "first_launch_ai_setup_openrouter_missing_key.yaml",
        "title_reopen_ai_setup.yaml",
    ] {
        let scenario = Scenario::from_path(scenario_dir().join(name)).unwrap();
        runner
            .run(&scenario)
            .unwrap_or_else(|error| panic!("scenario {name} failed: {error}"));
    }
}
