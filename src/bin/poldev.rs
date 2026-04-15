use polit::devtools::in_process::InProcessRunner;
use polit::devtools::pty::PtyRunner;
use polit::devtools::scenario::Scenario;
use polit::devtools::scenario::ScenarioMode;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match parse_args(std::env::args().skip(1))? {
        Command::Run { path } => {
            let scenario = Scenario::from_path(&path)?;
            let result = match scenario.mode {
                ScenarioMode::Pty => {
                    let binary = find_polit_binary()?;
                    PtyRunner::new(binary).run(&scenario)?
                }
                ScenarioMode::InProcess | ScenarioMode::Both => {
                    let result = InProcessRunner::new().run(&scenario)?;
                    println!("loaded scenario '{}' in {}", scenario.name, scenario.mode);
                    println!("{}", result.final_text.join("\n"));
                    return Ok(());
                }
            };
            println!("loaded scenario '{}' in {}", scenario.name, scenario.mode);
            println!("{}", result.final_text.join("\n"));
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
    Run { path: String },
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

    let Some(path) = args.next() else {
        return Err(UsageError);
    };

    Ok(Command::Run { path })
}

fn print_usage() {
    eprintln!("usage: poldev tui run <scenario.yaml>");
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

#[cfg(test)]
mod tests {
    use super::{parse_args, Command};

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
                path: "tests/tui/scenarios/smoke.yaml".to_string()
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
}
