use crossbeam_channel::{bounded, unbounded, Receiver, Sender};

use super::events::GameEvent;

/// Commands sent from UI thread → Game thread
#[derive(Debug, Clone)]
pub enum UiCommand {
    /// Player typed free text
    PlayerInput(String),
    /// Slash command parsed
    SlashCommand { cmd: String, args: Vec<String> },
    /// End the current turn
    EndTurn,
    /// Save game
    SaveGame(String),
    /// Load game
    LoadGame(String),
    /// Quit
    Quit,
}

/// Messages sent from Game thread → UI thread
#[derive(Debug, Clone)]
pub enum UiMessage {
    /// Add narration text to chat
    Narrate(String),
    /// NPC dialogue
    NpcDialogue { name: String, text: String },
    /// System message (yellow)
    System(String),
    /// Warning message (red)
    Warning(String),
    /// Success message (green)
    Success(String),
    /// Dice roll result
    DiceRoll(String),
    /// Phase header separator
    PhaseHeader(String),
    /// Update status bar
    StatusUpdate {
        week: u32,
        year: u32,
        phase: String,
        ap_current: i32,
        ap_max: i32,
    },
    /// Game event for overlays/tracking
    Event(GameEvent),
    /// Game is shutting down
    Shutdown,
}

/// Channel bundle for inter-thread communication
pub struct Channels {
    /// UI → Game commands
    pub cmd_tx: Sender<UiCommand>,
    pub cmd_rx: Receiver<UiCommand>,
    /// Game → UI messages
    pub msg_tx: Sender<UiMessage>,
    pub msg_rx: Receiver<UiMessage>,
}

impl Channels {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = bounded(64);
        let (msg_tx, msg_rx) = unbounded();
        Self {
            cmd_tx,
            cmd_rx,
            msg_tx,
            msg_rx,
        }
    }

    /// Split into UI-side and Game-side handles
    pub fn split(self) -> (UiChannels, GameChannels) {
        let ui = UiChannels {
            cmd_tx: self.cmd_tx,
            msg_rx: self.msg_rx,
        };
        let game = GameChannels {
            cmd_rx: self.cmd_rx,
            msg_tx: self.msg_tx,
        };
        (ui, game)
    }
}

/// Channels held by the UI thread
pub struct UiChannels {
    pub cmd_tx: Sender<UiCommand>,
    pub msg_rx: Receiver<UiMessage>,
}

/// Channels held by the Game thread
pub struct GameChannels {
    pub cmd_rx: Receiver<UiCommand>,
    pub msg_tx: Sender<UiMessage>,
}

impl GameChannels {
    pub fn send(&self, msg: UiMessage) {
        let _ = self.msg_tx.send(msg);
    }

    pub fn try_recv(&self) -> Option<UiCommand> {
        self.cmd_rx.try_recv().ok()
    }
}

impl UiChannels {
    pub fn send(&self, cmd: UiCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    pub fn drain_messages(&self) -> Vec<UiMessage> {
        let mut msgs = Vec::new();
        while let Ok(msg) = self.msg_rx.try_recv() {
            msgs.push(msg);
        }
        msgs
    }
}
