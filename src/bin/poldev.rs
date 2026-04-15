use polit::devtools::in_process::InProcessRunner;
use polit::devtools::pty::PtyRunner;
use polit::devtools::scenario::Scenario;
use polit::devtools::scenario::ScenarioMode;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match parse_args(std::env::args().skip(1))? {
        Command::Run {
            path,
            mode_override,
        } => {
            let scenario = Scenario::from_path(&path)?;
            let modes = requested_modes(mode_override, scenario.mode);
            let pty_binary = if modes.iter().any(|mode| *mode == ScenarioMode::Pty) {
                Some(find_polit_binary()?)
            } else {
                None
            };

            for mode in modes {
                let mut run_scenario = scenario.clone();
                run_scenario.mode = mode;
                let final_text = match mode {
                    ScenarioMode::InProcess => InProcessRunner::new().run(&run_scenario)?.final_text,
                    ScenarioMode::Pty => PtyRunner::new(
                        pty_binary
                            .as_ref()
                            .expect("pty binary should be resolved before loop"),
                    )
                    .run(&run_scenario)?
                    .final_text,
                    ScenarioMode::Both => unreachable!("requested modes never include both"),
                };

                println!("loaded scenario '{}' in {}", scenario.name, mode);
                println!("{}", final_text.join("\n"));
            }
            Ok(())
        }
        Command::Help => {
            print_usage();
            Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Run {
        path: String,
        mode_override: Option<ScenarioMode>,
    },
    Help,
}

#[derive(Debug)]
struct UsageError;

impl std::fmt::Display for UsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid poldev invocation")
    }
}

impl std::error::Error for UsageError {}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Command, UsageError> {
    let Some(first) = args.next() else {
        return Ok(Command::Help);
    };

    if first == "--help" || first == "-h" {
        return Ok(Command::Help);
    }

    if first != "tui" {
        return Err(UsageError);
    }

    let Some(second) = args.next() else {
        return Err(UsageError);
    };

    if second != "run" {
        return Err(UsageError);
    }

    let mut mode_override = None;
    let mut path = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mode" => {
                let Some(value) = args.next() else {
                    return Err(UsageError);
                };
                mode_override = Some(parse_mode_override(&value)?);
            }
            value if value.starts_with("--") => return Err(UsageError),
            value => {
                if path.is_some() {
                    return Err(UsageError);
                }
                path = Some(value.to_string());
            }
        }
    }

    let Some(path) = path else {
        return Err(UsageError);
    };

    Ok(Command::Run {
        path,
        mode_override,
    })
}

fn print_usage() {
    eprintln!("usage: poldev tui run [--mode <in_process|pty>] <scenario.yaml>");
}

fn find_polit_binary() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("POLDEV_POLIT_BIN") {
        return Ok(path.into());
    }

    let current = std::env::current_exe()?;
    let sibling = if cfg!(windows) {
        current.with_file_name("polit.exe")
    } else {
        current.with_file_name("polit")
    };

    if sibling.exists() {
        Ok(sibling)
    } else {
        Err("unable to locate polit binary; set POLDEV_POLIT_BIN".into())
    }
}

fn parse_mode_override(value: &str) -> Result<ScenarioMode, UsageError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "in_process" | "in-process" => Ok(ScenarioMode::InProcess),
        "pty" => Ok(ScenarioMode::Pty),
        "both" => Ok(ScenarioMode::Both),
        _ => Err(UsageError),
    }
}

fn requested_modes(
    mode_override: Option<ScenarioMode>,
    scenario_mode: ScenarioMode,
) -> Vec<ScenarioMode> {
    match mode_override.unwrap_or(scenario_mode) {
        ScenarioMode::InProcess => vec![ScenarioMode::InProcess],
        ScenarioMode::Pty => vec![ScenarioMode::Pty],
        ScenarioMode::Both => vec![ScenarioMode::InProcess, ScenarioMode::Pty],
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_args, requested_modes, Command};
    use polit::devtools::scenario::ScenarioMode;

    #[test]
    fn parses_tui_run_command() {
        let command = parse_args(
            ["tui", "run", "tests/tui/scenarios/smoke.yaml"]
                .into_iter()
                .map(str::to_string),
        )
        .unwrap();

        assert_eq!(
            command,
            Command::Run {
                path: "tests/tui/scenarios/smoke.yaml".to_string(),
                mode_override: None,
            }
        );
    }

    #[test]
    fn rejects_incomplete_command_shape() {
        let error = parse_args(["tui"].into_iter().map(str::to_string)).unwrap_err();

        assert_eq!(error.to_string(), "invalid poldev invocation");
    }

    #[test]
    fn help_flag_still_returns_help() {
        let command = parse_args(["--help"].into_iter().map(str::to_string)).unwrap();

        assert_eq!(command, Command::Help);
    }

    #[test]
    fn parses_mode_override() {
        let command = parse_args(
            ["tui", "run", "--mode", "pty", "tests/tui/scenarios/smoke.yaml"]
                .into_iter()
                .map(str::to_string),
        )
        .unwrap();

        assert_eq!(
            command,
            Command::Run {
                path: "tests/tui/scenarios/smoke.yaml".to_string(),
                mode_override: Some(ScenarioMode::Pty),
            }
        );
    }

    #[test]
    fn expands_both_mode_into_both_backends() {
        assert_eq!(
            requested_modes(None, ScenarioMode::Both),
            vec![ScenarioMode::InProcess, ScenarioMode::Pty]
        );
        assert_eq!(
            requested_modes(Some(ScenarioMode::Pty), ScenarioMode::Both),
            vec![ScenarioMode::Pty]
        );
    }
}
