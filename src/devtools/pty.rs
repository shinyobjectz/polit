use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::devtools::diagnostics::{collect_input_history, format_failure};
use crate::devtools::scenario::{
    Scenario, ScenarioFileExpectation, ScenarioMode, ScenarioSeedFile, ScenarioStep,
};
use crate::mcp::pty_session::{PtySession, PtySessionConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyRunResult {
    pub final_text: Vec<String>,
    pub snapshots: HashMap<String, Vec<String>>,
}

pub struct PtyRunner {
    binary_path: PathBuf,
}

const STEP_TIMEOUT: Duration = Duration::from_secs(2);
const INPUT_SETTLE_TIMEOUT: Duration = Duration::from_millis(750);
const QUIET_WINDOW: Duration = Duration::from_millis(100);

impl PtyRunner {
    pub fn new(binary_path: impl Into<PathBuf>) -> Self {
        Self {
            binary_path: binary_path.into(),
        }
    }

    pub fn run(&self, scenario: &Scenario) -> Result<PtyRunResult, Box<dyn std::error::Error>> {
        match scenario.mode {
            ScenarioMode::Pty | ScenarioMode::Both => {}
            ScenarioMode::InProcess => {
                return Err("scenario is marked for in-process mode only".into())
            }
        }

        if scenario.startup.command != "app" {
            return Err(format!(
                "pty runner does not yet support startup command '{}'",
                scenario.startup.command
            )
            .into());
        }

        let env = PtyTempEnv::new()?;
        apply_seed_files(env.home_path(), &scenario.fixtures.seed_files)?;
        if scenario.fixtures.fake_codex {
            env.install_fake_codex()?;
        }

        let mut session = PtySession::launch(
            &self.binary_path,
            PtySessionConfig::new(env.home_path(), scenario.terminal.width, scenario.terminal.height)
                .with_path_env(env.path_env()),
        )?;
        let mut snapshots = HashMap::new();

        for (step_offset, step) in scenario.steps.iter().enumerate() {
            let step_index = step_offset + 1;
            match step {
                ScenarioStep::Press { press } => {
                    session.send_key(press, INPUT_SETTLE_TIMEOUT)?;
                }
                ScenarioStep::Type { type_text } => {
                    session.type_text(type_text, INPUT_SETTLE_TIMEOUT)?;
                }
                ScenarioStep::AssertText { assert_text } => {
                    session.wait_for_text(assert_text, STEP_TIMEOUT).map_err(
                        |error: Box<dyn std::error::Error>| -> Box<dyn std::error::Error> {
                            format_failure(
                                scenario,
                                ScenarioMode::Pty,
                                Some(step_index),
                                &error.to_string(),
                                &session.screen_lines(),
                                &collect_input_history(&scenario.steps, step_index),
                                env.home_path(),
                            )
                            .into()
                        },
                    )?;
                }
                ScenarioStep::AssertNotText { assert_not_text } => {
                    session.settle_for(STEP_TIMEOUT);
                    let lines = session.screen_lines();
                    if lines.iter().any(|line| line.contains(assert_not_text)) {
                        return Err(format_failure(
                            scenario,
                            ScenarioMode::Pty,
                            Some(step_index),
                            &format!("unexpected text '{assert_not_text}' found in PTY screen"),
                            &lines,
                            &collect_input_history(&scenario.steps, step_index),
                            env.home_path(),
                        )
                        .into());
                    }
                }
                ScenarioStep::Snapshot { snapshot } => {
                    session.settle_for(STEP_TIMEOUT);
                    snapshots.insert(snapshot.clone(), session.screen_lines());
                }
            }
        }

        let _running = wait_for_child_state(
            session.child_mut(),
            scenario.expect.running,
            allows_non_success_exit(&scenario.steps),
        )
        .map_err(|error| -> Box<dyn std::error::Error> {
            format_failure(
                scenario,
                ScenarioMode::Pty,
                None,
                &error.to_string(),
                &session.screen_lines(),
                &collect_input_history(&scenario.steps, scenario.steps.len()),
                env.home_path(),
            )
            .into()
        })?;
        session.settle_for(QUIET_WINDOW);

        validate_expected_files(env.home_path(), &scenario.expect.files).map_err(
            |error| -> Box<dyn std::error::Error> {
                format_failure(
                    scenario,
                    ScenarioMode::Pty,
                    None,
                    &error.to_string(),
                    &session.screen_lines(),
                    &collect_input_history(&scenario.steps, scenario.steps.len()),
                    env.home_path(),
                )
                .into()
            },
        )?;

        Ok(PtyRunResult {
            final_text: session.screen_lines(),
            snapshots,
        })
    }
}

fn wait_for_child_state(
    child: &mut dyn portable_pty::Child,
    expect_running: bool,
    allow_non_success_exit: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(2);

    loop {
        if let Some(status) = child.try_wait()? {
            if expect_running {
                return Err("pty runner expected process to still be running".into());
            }
            if !status.success() && !allow_non_success_exit {
                return Err(format!(
                    "pty runner saw non-success exit status {}",
                    status.exit_code()
                )
                .into());
            }
            return Ok(false);
        }

        if Instant::now() >= deadline {
            if expect_running {
                child.kill()?;
                return Ok(true);
            }

            child.kill()?;
            return Err("pty runner expected process to exit but it was still running".into());
        }

        std::thread::sleep(QUIET_WINDOW);
    }
}

fn allows_non_success_exit(steps: &[ScenarioStep]) -> bool {
    steps.iter().any(|step| {
        matches!(
            step,
            ScenarioStep::Press { press }
                if matches!(press.trim().to_ascii_lowercase().as_str(), "ctrl-c" | "control-c")
        )
    })
}

fn apply_seed_files(
    home: &Path,
    seed_files: &[ScenarioSeedFile],
) -> Result<(), Box<dyn std::error::Error>> {
    for seed_file in seed_files {
        let path = home.join(&seed_file.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &seed_file.content)?;
    }

    Ok(())
}

fn validate_expected_files(
    home: &Path,
    file_expectations: &[ScenarioFileExpectation],
) -> Result<(), Box<dyn std::error::Error>> {
    for expectation in file_expectations {
        let path = home.join(&expectation.path);
        let content = fs::read_to_string(&path).map_err(|error| {
            format!("failed to read expected file {}: {}", path.display(), error)
        })?;
        if !content.contains(&expectation.contains) {
            return Err(format!(
                "expected file {} to contain {:?}",
                path.display(),
                expectation.contains
            )
            .into());
        }
    }

    Ok(())
}

struct PtyTempEnv {
    home: tempfile::TempDir,
    path_entries: std::cell::RefCell<Vec<PathBuf>>,
    inherited_path: String,
}

impl PtyTempEnv {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            home: tempfile::TempDir::new()?,
            path_entries: std::cell::RefCell::new(Vec::new()),
            inherited_path: std::env::var("PATH").unwrap_or_default(),
        })
    }

    fn home_path(&self) -> &Path {
        self.home.path()
    }

    fn install_fake_codex(&self) -> Result<(), Box<dyn std::error::Error>> {
        let bin_dir = self.home_path().join(".poldev").join("bin");
        fs::create_dir_all(&bin_dir)?;
        let binary_path = fake_codex_binary_path(&bin_dir);
        fs::write(&binary_path, fake_codex_script())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = fs::metadata(&binary_path)?.permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&binary_path, permissions)?;
        }

        // This mutation is isolated to the child process env via `path_env`.
        let mut entries = self.path_entries.borrow_mut();
        if !entries.iter().any(|existing| existing == &bin_dir) {
            entries.insert(0, bin_dir);
        }
        Ok(())
    }

    fn path_env(&self) -> String {
        let mut entries: Vec<String> = self
            .path_entries
            .borrow()
            .iter()
            .map(|path| path.display().to_string())
            .collect();
        if !self.inherited_path.is_empty() {
            entries.push(self.inherited_path.clone());
        }
        entries.join(if cfg!(windows) { ";" } else { ":" })
    }
}

fn fake_codex_binary_path(bin_dir: &Path) -> PathBuf {
    if cfg!(windows) {
        bin_dir.join("codex.cmd")
    } else {
        bin_dir.join("codex")
    }
}

fn fake_codex_script() -> &'static str {
    if cfg!(windows) {
        "@echo off\r\nif \"%1\"==\"login\" if \"%2\"==\"status\" (\r\n  echo Logged in using ChatGPT\r\n  exit /b 0\r\n)\r\necho unexpected codex args %* 1>&2\r\nexit /b 1\r\n"
    } else {
        "#!/bin/sh\nif [ \"$1\" = \"login\" ] && [ \"$2\" = \"status\" ]; then\n  echo \"Logged in using ChatGPT\"\n  exit 0\nfi\necho \"unexpected codex args: $*\" >&2\nexit 1\n"
    }
}
