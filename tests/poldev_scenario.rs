use polit::devtools::scenario::{Scenario, ScenarioStep};

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
  - mash: enter
expect:
  running: true
"#;

    let error = Scenario::from_yaml(yaml).unwrap_err();
    let message = error.to_string();

    assert!(message.contains("mash"), "unexpected error: {message}");
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

#[test]
fn parses_runner_steps() {
    let yaml = r#"
name: smoke
mode: in_process
terminal:
  width: 120
  height: 40
startup:
  command: app
steps:
  - press: ctrl-c
  - type: "hello"
  - assert_not_text: "Error"
  - snapshot: "final"
expect:
  running: true
"#;

    let scenario = Scenario::from_yaml(yaml).unwrap();

    assert_eq!(scenario.steps.len(), 4);
    assert!(matches!(
        &scenario.steps[0],
        ScenarioStep::Press { press } if press == "ctrl-c"
    ));
    assert!(matches!(
        &scenario.steps[1],
        ScenarioStep::Type { type_text } if type_text == "hello"
    ));
}

#[test]
fn parses_fixtures_and_file_expectations() {
    let yaml = r#"
name: smoke
mode: in_process
fixtures:
  fake_codex: true
  seed_files:
    - path: ".polit/config/ai.toml"
      content: |
        provider = "codex"
terminal:
  width: 120
  height: 40
startup:
  command: title
  has_save: true
steps:
  - assert_text: "POLIT"
expect:
  running: false
  files:
    - path: ".polit/config/ai.toml"
      contains: 'provider = "codex"'
"#;

    let scenario = Scenario::from_yaml(yaml).unwrap();

    assert!(scenario.fixtures.fake_codex);
    assert_eq!(scenario.fixtures.seed_files.len(), 1);
    assert_eq!(scenario.startup.command, "title");
    assert!(scenario.startup.has_save);
    assert_eq!(scenario.expect.files.len(), 1);
}
