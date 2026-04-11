//! Structured debug log for agent turns.
//! Writes JSONL to ~/.polit/agent_debug.jsonl so every agent interaction
//! can be reviewed after a run.

use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;



/// A single logged agent turn
#[derive(Debug, Serialize)]
pub struct AgentTurnLog {
    pub timestamp: String,
    pub mode: String,
    pub user_input: String,
    pub prompt_chars: usize,
    pub prompt_est_tokens: usize,
    pub raw_output: String,
    pub parsed_narration: String,
    pub parsed_tool_calls: Vec<String>,
    pub duration_ms: u64,
    pub iterations: u32,
    pub memory_turns: u32,
    pub character_fields: Vec<(String, String)>,
}

/// Global debug logger — writes to ~/.polit/agent_debug.jsonl
pub struct DebugLog {
    path: PathBuf,
}

static DEBUG_LOG: Mutex<Option<DebugLog>> = Mutex::new(None);

impl DebugLog {
    /// Initialize the debug log (call once at startup)
    pub fn init() {
        if let Some(home) = std::env::var_os("HOME") {
            let path = PathBuf::from(home).join(".polit").join("agent_debug.jsonl");
            let mut lock = DEBUG_LOG.lock().unwrap();
            *lock = Some(DebugLog { path });
        }
    }

    /// Log a completed agent turn
    pub fn log_turn(entry: &AgentTurnLog) {
        let lock = DEBUG_LOG.lock().unwrap();
        if let Some(ref log) = *lock {
            if let Ok(json) = serde_json::to_string(entry) {
                if let Ok(mut file) = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log.path)
                {
                    let _ = writeln!(file, "{}", json);
                }
            }
        }
    }

    /// Get the log file path for display
    pub fn path() -> Option<PathBuf> {
        let lock = DEBUG_LOG.lock().unwrap();
        lock.as_ref().map(|l| l.path.clone())
    }
}

/// Timer for measuring agent turn duration
pub struct TurnTimer {
    start: Instant,
}

impl TurnTimer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}
