use polit::devtools::in_process::InProcessRunner;
use polit::devtools::pty::PtyRunner;
use polit::devtools::scenario::Scenario;
use polit::devtools::scenario::ScenarioMode;

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

#[test]
fn first_launch_codex_scenario_runs_in_both_backends() {
    let scenario_path = scenario_dir().join("first_launch_ai_setup_codex.yaml");
    let scenario = Scenario::from_path(&scenario_path).unwrap();

    assert_eq!(scenario.mode, ScenarioMode::Both);

    let in_process = InProcessRunner::new().run(&scenario).unwrap();
    assert!(in_process
        .final_text
        .iter()
        .any(|line| line.contains("AI Setup") || line.contains("Validate Codex and save")));

    let binary = std::env::var("CARGO_BIN_EXE_polit").expect("polit binary path");
    let pty = PtyRunner::new(binary).run(&scenario).unwrap();
    assert!(pty
        .final_text
        .iter()
        .any(|line| line.contains("AI Setup") || line.contains("Validate Codex and save")));
}
