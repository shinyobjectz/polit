use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn failing_cli_run_reports_scenario_backend_step_and_frame() {
    let dir = tempdir().unwrap();
    let scenario_path = dir.path().join("failing.yaml");
    fs::write(
        &scenario_path,
        r#"
name: failing_cli
mode: in_process
terminal:
  width: 100
  height: 30
startup:
  command: app
steps:
  - assert_text: "AI Setup"
  - assert_text: "text that does not exist"
  - press: ctrl-c
expect:
  running: false
"#,
    )
    .unwrap();

    let output = Command::new(std::env::var("CARGO_BIN_EXE_poldev").unwrap())
        .args(["tui", "run", "--mode", "in_process"])
        .arg(&scenario_path)
        .output()
        .unwrap();

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failing_cli"));
    assert!(stderr.contains("in_process"));
    assert!(stderr.contains("step 2"));
    assert!(stderr.contains("AI Setup"));
}
