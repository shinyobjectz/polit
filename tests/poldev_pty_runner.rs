use polit::devtools::pty::PtyRunner;
use polit::devtools::scenario::Scenario;

#[test]
fn poldev_pty_runner_captures_startup_screen_from_real_binary() {
    let scenario = Scenario::from_yaml(
        r#"
name: pty_startup_smoke
mode: pty
fixtures:
  fake_codex: true
terminal:
  width: 100
  height: 30
startup:
  command: app
steps:
  - assert_text: "AI Setup"
  - snapshot: "startup"
  - press: ctrl-c
expect:
  running: false
"#,
    )
    .unwrap();

    let binary = std::env::var("CARGO_BIN_EXE_polit").expect("polit binary path");
    let result = PtyRunner::new(binary).run(&scenario).unwrap();

    assert!(result
        .snapshots
        .get("startup")
        .unwrap()
        .iter()
        .any(|line| line.contains("AI Setup")));
}
