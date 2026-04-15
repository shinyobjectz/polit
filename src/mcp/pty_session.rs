use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

const QUIET_WINDOW: Duration = Duration::from_millis(100);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub struct PtySessionConfig {
    pub home_path: PathBuf,
    pub terminal_width: u16,
    pub terminal_height: u16,
    pub path_env: Option<String>,
}

impl PtySessionConfig {
    pub fn new(home_path: impl Into<PathBuf>, terminal_width: u16, terminal_height: u16) -> Self {
        Self {
            home_path: home_path.into(),
            terminal_width,
            terminal_height,
            path_env: None,
        }
    }

    pub fn with_path_env(mut self, path_env: impl Into<String>) -> Self {
        self.path_env = Some(path_env.into());
        self
    }
}

pub struct PtySession {
    master: Box<dyn portable_pty::MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    receiver: mpsc::Receiver<Vec<u8>>,
    parser: vt100::Parser,
    last_screen_text: String,
    screen_revision: u64,
}

impl PtySession {
    pub fn launch(
        binary_path: impl AsRef<Path>,
        config: PtySessionConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let pty_system = NativePtySystem::default();
        let pair = pty_system.openpty(PtySize {
            rows: config.terminal_height,
            cols: config.terminal_width,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut command = CommandBuilder::new(binary_path.as_ref());
        command.env("HOME", &config.home_path);
        if let Some(path_env) = &config.path_env {
            command.env("PATH", path_env);
        }

        let child = pair.slave.spawn_command(command)?;
        let reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;

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

        let mut session = Self {
            master: pair.master,
            child,
            writer,
            receiver: rx,
            parser: vt100::Parser::new(config.terminal_height, config.terminal_width, 0),
            last_screen_text: String::new(),
            screen_revision: 0,
        };
        session.settle_for(STARTUP_TIMEOUT);
        Ok(session)
    }

    pub fn send_key(
        &mut self,
        press: &str,
        settle_timeout: Duration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.writer.write_all(&press_bytes(press)?)?;
        self.writer.flush()?;
        self.settle_for(settle_timeout);
        Ok(())
    }

    pub fn type_text(
        &mut self,
        text: &str,
        settle_timeout: Duration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for ch in text.chars() {
            self.writer.write_all(&type_bytes(ch))?;
            self.writer.flush()?;
            self.settle_for(settle_timeout);
        }

        Ok(())
    }

    pub fn wait_for_text(
        &mut self,
        expected: &str,
        timeout: Duration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let deadline = Instant::now() + timeout;

        loop {
            if self
                .screen_lines()
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
            match self.receiver.recv_timeout(wait) {
                Ok(chunk) => self.process_chunk(&chunk),
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

    pub fn settle_for(&mut self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        let mut saw_output = false;

        loop {
            let timeout = deadline.saturating_duration_since(Instant::now());
            if timeout.is_zero() {
                break;
            }

            match self.receiver.recv_timeout(timeout.min(QUIET_WINDOW)) {
                Ok(chunk) => {
                    self.process_chunk(&chunk);
                    saw_output = true;
                }
                Err(mpsc::RecvTimeoutError::Timeout) if saw_output => break,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    pub fn screen_lines(&self) -> Vec<String> {
        self.parser
            .screen()
            .contents()
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect()
    }

    pub fn screen_revision(&self) -> u64 {
        self.screen_revision
    }

    pub fn child_mut(&mut self) -> &mut (dyn portable_pty::Child + Send + Sync) {
        &mut *self.child
    }

    pub fn terminate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.child.try_wait()?.is_none() {
            self.child.kill()?;
        }
        Ok(())
    }

    pub fn terminal_size(&self) -> Result<(u16, u16), Box<dyn std::error::Error>> {
        let size = self.master.get_size()?;
        Ok((size.cols, size.rows))
    }

    fn process_chunk(&mut self, chunk: &[u8]) {
        self.parser.process(chunk);
        let screen = self.parser.screen().contents().to_string();
        if screen != self.last_screen_text {
            self.screen_revision += 1;
            self.last_screen_text = screen;
        }
    }
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
