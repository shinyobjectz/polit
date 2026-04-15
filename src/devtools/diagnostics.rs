use std::path::Path;

use crate::devtools::scenario::{Scenario, ScenarioMode, ScenarioStep};

pub fn format_failure(
    scenario: &Scenario,
    mode: ScenarioMode,
    step_index: Option<usize>,
    message: &str,
    frame: &[String],
    input_history: &[String],
    temp_home: &Path,
) -> String {
    let mut lines = vec![format!("scenario '{}' failed in {}", scenario.name, mode)];

    if let Some(step_index) = step_index {
        lines.push(format!(
            "failing step {}: {}",
            step_index,
            describe_step(&scenario.steps[step_index.saturating_sub(1)])
        ));
    }

    lines.push(format!("error: {message}"));
    lines.push(format!("temp home: {}", temp_home.display()));

    if input_history.is_empty() {
        lines.push("recent input: none".to_string());
    } else {
        lines.push("recent input:".to_string());
        for entry in input_history.iter().rev().take(8).rev() {
            lines.push(format!("  {entry}"));
        }
    }

    lines.push("frame:".to_string());
    if frame.is_empty() {
        lines.push("  <empty>".to_string());
    } else {
        for line in frame {
            lines.push(format!("  {line}"));
        }
    }

    lines.join("\n")
}

pub fn collect_input_history(steps: &[ScenarioStep], through_step: usize) -> Vec<String> {
    let mut history = Vec::new();

    for (index, step) in steps.iter().take(through_step).enumerate() {
        match step {
            ScenarioStep::Press { press } => {
                history.push(format!("step {} press {}", index + 1, press));
            }
            ScenarioStep::Type { type_text } => {
                history.push(format!("step {} type {:?}", index + 1, type_text));
            }
            ScenarioStep::AssertText { .. }
            | ScenarioStep::AssertNotText { .. }
            | ScenarioStep::Snapshot { .. } => {}
        }
    }

    history
}

fn describe_step(step: &ScenarioStep) -> String {
    match step {
        ScenarioStep::AssertText { assert_text } => format!("assert_text {:?}", assert_text),
        ScenarioStep::AssertNotText { assert_not_text } => {
            format!("assert_not_text {:?}", assert_not_text)
        }
        ScenarioStep::Press { press } => format!("press {:?}", press),
        ScenarioStep::Type { type_text } => format!("type {:?}", type_text),
        ScenarioStep::Snapshot { snapshot } => format!("snapshot {:?}", snapshot),
    }
}
