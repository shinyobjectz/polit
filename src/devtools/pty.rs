use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

use crate::devtools::scenario::{
    Scenario, ScenarioFileExpectation, ScenarioMode, ScenarioSeedFile, ScenarioStep,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyRunResult {
    pub final_text: Vec<String>,
    pub snapshots: HashMap<String, Vec<String>>,
}

pub struct PtyRunner {
    binary_path: PathBuf,
}

const STARTUP_TIMEOUT: Duration = Duration::from_secs(2);
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

        let pty_system = NativePtySystem::default();
        let pair = pty_system.openpty(PtySize {
            rows: scenario.terminal.height,
            cols: scenario.terminal.width,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut command = CommandBuilder::new(&self.binary_path);
        command.env("HOME", env.home_path());
        command.env("PATH", env.path_env());
        let mut child = pair.slave.spawn_command(command)?;

        let reader = pair.master.try_clone_reader()?;
        let mut writer = pair.master.take_writer()?;

        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let _reader_thread = std::thread::spawn(move || {
            let mut reader = reader;
            let mut buffer = [0u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        if tx.send(buffer[..count].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let mut parser = vt100::Parser::new(scenario.terminal.height, scenario.terminal.width, 0);
        let mut snapshots = HashMap::new();

        settle_screen(&rx, &mut parser, STARTUP_TIMEOUT);

        for step in &scenario.steps {
            match step {
                ScenarioStep::Press { press } => {
                    writer.write_all(&press_bytes(press)?)?;
                    writer.flush()?;
                    settle_screen(&rx, &mut parser, INPUT_SETTLE_TIMEOUT);
                }
                ScenarioStep::Type { type_text } => {
                    for ch in type_text.chars() {
                        writer.write_all(&type_bytes(ch))?;
                        writer.flush()?;
                        settle_screen(&rx, &mut parser, INPUT_SETTLE_TIMEOUT);
                    }
                }
                ScenarioStep::AssertText { assert_text } => {
                    wait_for_text(&rx, &mut parser, assert_text, STEP_TIMEOUT)?;
                }
                ScenarioStep::AssertNotText { assert_not_text } => {
                    settle_screen(&rx, &mut parser, STEP_TIMEOUT);
                    let lines = screen_lines(&parser);
                    if lines.iter().any(|line| line.contains(assert_not_text)) {
                        return Err(format!(
                            "unexpected text '{assert_not_text}' found in PTY screen"
                        )
                        .into());
                    }
                }
                ScenarioStep::Snapshot { snapshot } => {
                    settle_screen(&rx, &mut parser, STEP_TIMEOUT);
                    snapshots.insert(snapshot.clone(), screen_lines(&parser));
                }
            }
        }

        let _running = wait_for_child_state(
            child.as_mut(),
            scenario.expect.running,
            allows_non_success_exit(&scenario.steps),
        )?;
        settle_screen(&rx, &mut parser, QUIET_WINDOW);

        validate_expected_files(env.home_path(), &scenario.expect.files)?;

        Ok(PtyRunResult {
            final_text: screen_lines(&parser),
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

fn wait_for_text(
    receiver: &mpsc::Receiver<Vec<u8>>,
    parser: &mut vt100::Parser,
    expected: &str,
    timeout: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let deadline = Instant::now() + timeout;

    loop {
        if screen_lines(parser)
            .iter()
            .any(|line| line.contains(expected))
        {
            return Ok(());
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(format!("expected text '{expected}' not found in PTY screen").into());
        }

        let wait = remaining.min(QUIET_WINDOW);
        match receiver.recv_timeout(wait) {
            Ok(chunk) => parser.process(&chunk),
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(format!(
                    "pty output disconnected before text '{expected}' appeared"
                )
                .into())
            }
        }
    }
}

fn settle_screen(receiver: &mpsc::Receiver<Vec<u8>>, parser: &mut vt100::Parser, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    let mut saw_output = false;

    loop {
        let timeout = deadline.saturating_duration_since(Instant::now());
        if timeout.is_zero() {
            break;
        }

        match receiver.recv_timeout(timeout.min(QUIET_WINDOW)) {
            Ok(chunk) => {
                parser.process(&chunk);
                saw_output = true;
            }
            Err(mpsc::RecvTimeoutError::Timeout) if saw_output => break,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn screen_lines(parser: &vt100::Parser) -> Vec<String> {
    parser
        .screen()
        .contents()
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect()
}

fn press_bytes(press: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let normalized = press.trim().to_lowercase();
    let bytes = match normalized.as_str() {
        "enter" | "return" => vec![b'\r'],
        "tab" => vec![b'\t'],
        "esc" | "escape" => vec![0x1b],
        "up" => b"\x1b[A".to_vec(),
        "down" => b"\x1b[B".to_vec(),
        "right" => b"\x1b[C".to_vec(),
        "left" => b"\x1b[D".to_vec(),
        "backspace" => vec![0x7f],
        "ctrl-c" | "control-c" => vec![0x03],
        "ctrl-q" | "control-q" => vec![0x11],
        value if value.chars().count() == 1 => value.as_bytes().to_vec(),
        other => return Err(format!("unsupported press key '{other}'").into()),
    };

    Ok(bytes)
}

fn type_bytes(ch: char) -> Vec<u8> {
    match ch {
        '\n' => vec![b'\r'],
        '\t' => vec![b'\t'],
        c => c.to_string().into_bytes(),
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
