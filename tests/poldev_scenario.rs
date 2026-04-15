use polit::devtools::scenario::Scenario;

#[test]
fn parses_minimal_tui_scenario() {
    let yaml = r#"
name: smoke
mode: in_process
terminal:
  width: 120
  height: 40
startup:
  command: app
steps:
  - assert_text: "POLIT"
expect:
  running: true
"#;

    let scenario = Scenario::from_yaml(yaml).unwrap();

    assert_eq!(scenario.name, "smoke");
    assert_eq!(scenario.mode.to_string(), "in_process");
    assert_eq!(scenario.terminal.width, 120);
    assert_eq!(scenario.terminal.height, 40);
    assert_eq!(scenario.startup.command, "app");
    assert_eq!(scenario.steps.len(), 1);
    assert!(scenario.expect.running);
}

#[test]
fn rejects_unknown_mode() {
    let yaml = r#"
name: smoke
mode: mystery
terminal:
  width: 120
  height: 40
startup:
  command: app
steps:
  - assert_text: "POLIT"
expect:
  running: true
"#;

    let error = Scenario::from_yaml(yaml).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("mystery"), "unexpected error: {message}");
}

#[test]
fn rejects_unknown_step_shape() {
    let yaml = r#"
name: smoke
mode: in_process
terminal:
  width: 120
  height: 40
startup:
  command: app
steps:
  - press: enter
expect:
  running: true
"#;

    let error = Scenario::from_yaml(yaml).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("press"), "unexpected error: {message}");
}

#[test]
fn parses_both_mode() {
    let yaml = r#"
name: smoke
mode: both
terminal:
  width: 120
  height: 40
startup:
  command: app
steps:
  - assert_text: "POLIT"
expect:
  running: true
"#;

    let scenario = Scenario::from_yaml(yaml).unwrap();

    assert_eq!(scenario.mode.to_string(), "both");
}

#[test]
fn rejects_unknown_top_level_fields() {
    let yaml = r#"
name: smoke
mode: in_process
terminal:
  width: 120
  height: 40
startup:
  command: app
steps:
  - assert_text: "POLIT"
expect:
  running: true
typo_field: true
"#;

    let error = Scenario::from_yaml(yaml).unwrap_err();
    let message = error.to_string();

    assert!(
        message.contains("typo_field"),
        "unexpected error: {message}"
    );
}
