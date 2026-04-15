use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::{Backend, ClearType, TestBackend, WindowSize};
use ratatui::buffer::Cell;
use ratatui::layout::{Position, Size};
use ratatui::Terminal;
use tempfile::TempDir;

use crate::devtools::frame_dump::buffer_to_text_lines;
use crate::devtools::harness::ScriptedEventSource;
use crate::devtools::scenario::{Scenario, ScenarioMode, ScenarioStep};
use crate::ui::music::MusicController;
use crate::ui::setup::{run_setup_flow, SetupOutcome};
use crate::ui::title::{TitleAction, TitleScreen};
use crate::ui::{run_startup_gate, StartupGateOutcome};

#[derive(Debug, Default)]
pub struct InProcessRunner;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InProcessRunResult {
    pub final_text: Vec<String>,
    pub snapshots: HashMap<String, Vec<String>>,
}

impl InProcessRunner {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self, scenario: &Scenario) -> Result<InProcessRunResult, Box<dyn Error>> {
        match scenario.mode {
            ScenarioMode::InProcess | ScenarioMode::Both => {}
            ScenarioMode::Pty => {
                return Err("scenario is marked for pty mode only".into());
            }
        }

        let _home = TempHome::install()?;
        let ai_config_path = home_config_path(_home.path());
        apply_fixtures(&scenario, &_home)?;

        let backend = RecordingBackend::new(scenario.terminal.width, scenario.terminal.height);
        let mut terminal = Terminal::new(backend)?;
        let scripted_events = build_scripted_events(&scenario.steps)?;
        let mut events = ScriptedEventSource::new(scripted_events);

        let outcome = match scenario.startup.command.as_str() {
            "app" => run_startup_gate(&mut terminal, &mut events, &ai_config_path)?,
            "title" => run_title_setup_flow(
                &mut terminal,
                &mut events,
                &ai_config_path,
                scenario.startup.has_save,
            )?,
            other => {
                return Err(format!("unsupported startup command '{other}'").into());
            }
        };

        let final_text = buffer_to_text_lines(terminal.backend().buffer());
        let snapshots = evaluate_steps(&scenario.steps, terminal.backend().frames())?;
        validate_expected_files(_home.path(), &scenario.expect.files)?;

        let running = matches!(outcome, StartupGateOutcome::Continue);
        if running != scenario.expect.running {
            return Err(format!(
                "scenario expected running={} but runner produced running={}",
                scenario.expect.running, running
            )
            .into());
        }

        Ok(InProcessRunResult {
            final_text,
            snapshots,
        })
    }
}

fn run_title_setup_flow(
    terminal: &mut Terminal<RecordingBackend>,
    events: &mut ScriptedEventSource,
    ai_config_path: &Path,
    has_save: bool,
) -> Result<StartupGateOutcome, Box<dyn Error>> {
    let music = MusicController::start_anthem();

    loop {
        let mut title = TitleScreen::new(has_save);
        let action = title.run(terminal, &music, events)?;

        match action {
            TitleAction::Settings => {
                if run_setup_flow(terminal, events, ai_config_path.to_path_buf(), false, None)?
                    == SetupOutcome::Cancelled
                {
                    continue;
                }
            }
            TitleAction::Quit => {
                music.stop();
                return Ok(StartupGateOutcome::Cancelled);
            }
            TitleAction::NewCampaign | TitleAction::ContinueCampaign => {
                music.stop();
                return Ok(StartupGateOutcome::Continue);
            }
        }
    }
}

fn apply_fixtures(scenario: &Scenario, home: &TempHome) -> Result<(), Box<dyn Error>> {
    for seed_file in &scenario.fixtures.seed_files {
        let path = home.path().join(&seed_file.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &seed_file.content)?;
    }

    if scenario.fixtures.fake_codex {
        home.install_fake_codex()?;
    }

    Ok(())
}

fn validate_expected_files(
    home: &Path,
    file_expectations: &[crate::devtools::scenario::ScenarioFileExpectation],
) -> Result<(), Box<dyn Error>> {
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

#[derive(Debug, Clone)]
struct RecordingBackend {
    inner: TestBackend,
    frames: Vec<Vec<String>>,
}

impl RecordingBackend {
    fn new(width: u16, height: u16) -> Self {
        Self {
            inner: TestBackend::new(width, height),
            frames: Vec::new(),
        }
    }

    fn buffer(&self) -> &ratatui::buffer::Buffer {
        self.inner.buffer()
    }

    fn frames(&self) -> &[Vec<String>] {
        &self.frames
    }
}

impl Backend for RecordingBackend {
    fn draw<'a, I>(&mut self, content: I) -> std::io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        self.inner.draw(content)?;
        self.frames.push(buffer_to_text_lines(self.inner.buffer()));
        Ok(())
    }

    fn append_lines(&mut self, n: u16) -> std::io::Result<()> {
        self.inner.append_lines(n)
    }

    fn hide_cursor(&mut self) -> std::io::Result<()> {
        self.inner.hide_cursor()
    }

    fn show_cursor(&mut self) -> std::io::Result<()> {
        self.inner.show_cursor()
    }

    fn get_cursor_position(&mut self) -> std::io::Result<Position> {
        self.inner.get_cursor_position()
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> std::io::Result<()> {
        self.inner.set_cursor_position(position)
    }

    fn clear(&mut self) -> std::io::Result<()> {
        self.inner.clear()
    }

    fn clear_region(&mut self, clear_type: ClearType) -> std::io::Result<()> {
        self.inner.clear_region(clear_type)
    }

    fn size(&self) -> std::io::Result<Size> {
        self.inner.size()
    }

    fn window_size(&mut self) -> std::io::Result<WindowSize> {
        self.inner.window_size()
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn build_scripted_events(steps: &[ScenarioStep]) -> Result<Vec<Event>, Box<dyn Error>> {
    let mut events = Vec::new();

    for step in steps {
        match step {
            ScenarioStep::Press { press } => events.push(parse_press(press)?),
            ScenarioStep::Type { type_text } => {
                for ch in type_text.chars() {
                    events.push(match ch {
                        '\n' => Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
                        '\t' => Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
                        c => Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)),
                    });
                }
            }
            _ => {}
        }
    }

    Ok(events)
}

fn parse_press(press: &str) -> Result<Event, Box<dyn Error>> {
    let normalized = press.trim().to_lowercase();
    let event = match normalized.as_str() {
        "enter" | "return" => Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        "esc" | "escape" => Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        "tab" => Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
        "backspace" => Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
        "up" => Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
        "down" => Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        "left" => Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
        "right" => Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
        "ctrl-c" | "control-c" => {
            Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))
        }
        "ctrl-q" | "control-q" => {
            Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL))
        }
        value if value.chars().count() == 1 => {
            let ch = value.chars().next().expect("single char already checked");
            Event::Key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE))
        }
        other => return Err(format!("unsupported press key '{other}'").into()),
    };

    Ok(event)
}

fn evaluate_steps(
    steps: &[ScenarioStep],
    frames: &[Vec<String>],
) -> Result<HashMap<String, Vec<String>>, Box<dyn Error>> {
    if frames.is_empty() {
        return Err("scenario produced no frames".into());
    }

    let mut snapshots = HashMap::new();
    let mut frame_index = 0usize;

    for step in steps {
        match step {
            ScenarioStep::Press { .. } => {
                frame_index += 1;
            }
            ScenarioStep::Type { type_text } => {
                frame_index += typed_event_count(type_text);
            }
            ScenarioStep::AssertText { assert_text } => {
                let frame = current_frame(frames, frame_index)?;
                if !frame.iter().any(|line| line.contains(assert_text)) {
                    return Err(format!(
                        "expected text '{assert_text}' not found in frame {frame_index}"
                    )
                    .into());
                }
            }
            ScenarioStep::AssertNotText { assert_not_text } => {
                let frame = current_frame(frames, frame_index)?;
                if frame.iter().any(|line| line.contains(assert_not_text)) {
                    return Err(format!(
                        "unexpected text '{assert_not_text}' found in frame {frame_index}"
                    )
                    .into());
                }
            }
            ScenarioStep::Snapshot { snapshot } => {
                snapshots.insert(
                    snapshot.clone(),
                    current_frame(frames, frame_index)?.to_vec(),
                );
            }
        }
    }

    Ok(snapshots)
}

fn current_frame(frames: &[Vec<String>], frame_index: usize) -> Result<&[String], Box<dyn Error>> {
    frames
        .get(frame_index)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("frame {frame_index} was not captured").into())
}

fn typed_event_count(type_text: &str) -> usize {
    type_text.chars().count()
}

fn home_config_path(home: &std::path::Path) -> PathBuf {
    home.join(".polit").join("config").join("ai.toml")
}

struct TempHome {
    dir: TempDir,
    previous_home: Option<std::ffi::OsString>,
    previous_path: Option<std::ffi::OsString>,
    _env_lock: MutexGuard<'static, ()>,
}

impl TempHome {
    fn install() -> Result<Self, Box<dyn Error>> {
        let env_lock = home_env_lock()
            .lock()
            .map_err(|_| "failed to acquire HOME environment lock")?;
        let dir = TempDir::new()?;
        let previous_home = env::var_os("HOME");
        env::set_var("HOME", dir.path());

        Ok(Self {
            dir,
            previous_home,
            previous_path: env::var_os("PATH"),
            _env_lock: env_lock,
        })
    }

    fn path(&self) -> &std::path::Path {
        self.dir.path()
    }

    fn install_fake_codex(&self) -> Result<(), Box<dyn Error>> {
        let bin_dir = self.path().join(".poldev").join("bin");
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

        let existing_path = self
            .previous_path
            .as_ref()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default();
        let separator = if existing_path.is_empty() {
            ""
        } else if cfg!(windows) {
            ";"
        } else {
            ":"
        };
        env::set_var(
            "PATH",
            format!("{}{}{}", bin_dir.display(), separator, existing_path),
        );

        Ok(())
    }
}

fn home_env_lock() -> &'static Mutex<()> {
    static HOME_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    HOME_ENV_LOCK.get_or_init(|| Mutex::new(()))
}

impl Drop for TempHome {
    fn drop(&mut self) {
        if let Some(previous_home) = &self.previous_home {
            env::set_var("HOME", previous_home);
        } else {
            env::remove_var("HOME");
        }

        if let Some(previous_path) = &self.previous_path {
            env::set_var("PATH", previous_path);
        } else {
            env::remove_var("PATH");
        }
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
