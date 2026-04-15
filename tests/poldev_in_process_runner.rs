use polit::devtools::in_process::InProcessRunner;
use polit::devtools::scenario::Scenario;
use std::fs;
use tempfile::tempdir;

#[test]
fn poldev_in_process_runner_loads_yaml_and_exercises_real_startup_path() {
    let dir = tempdir().unwrap();
    let scenario_path = dir.path().join("first_launch_setup.yaml");
    fs::write(
        &scenario_path,
        r#"
name: first_launch_setup
mode: in_process
terminal:
  width: 100
  height: 30
startup:
  command: app
steps:
  - snapshot: "codex-default"
  - assert_text: "Not needed for Codex."
  - press: right
  - assert_text: "AI Setup"
  - assert_text: "openrouter/deepseek-r1"
  - assert_not_text: "Not needed for Codex."
  - snapshot: "openrouter-selected"
  - press: ctrl-c
expect:
  running: false
"#,
    )
    .unwrap();

    let scenario = Scenario::from_path(&scenario_path).unwrap();
    let result = InProcessRunner::new().run(&scenario).unwrap();

    assert!(result
        .final_text
        .iter()
        .any(|line| line.contains("AI Setup")));

    let codex_snapshot = result.snapshots.get("codex-default").unwrap();
    assert!(codex_snapshot
        .iter()
        .any(|line| line.contains("Not needed for Codex.")));

    let openrouter_snapshot = result.snapshots.get("openrouter-selected").unwrap();
    assert!(openrouter_snapshot
        .iter()
        .any(|line| line.contains("openrouter/deepseek-r1")));
    assert!(!openrouter_snapshot
        .iter()
        .any(|line| line.contains("Not needed for Codex.")));
}
