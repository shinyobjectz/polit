use serde::{Deserialize, Serialize};

/// Game events that flow through the event bus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    // Narrative
    Narrate { text: String },
    NpcDialogue { npc_id: String, text: String },

    // World changes
    WeekAdvanced { week: u32, year: u32 },
    PhaseChanged { phase: String },

    // Character events
    CardAcquired { card_id: String },
    CardRevoked { card_id: String },
    CardPlayed { card_id: String },
    RelationshipChanged { npc_id: String, field: String, delta: i32 },

    // Political events
    LawProposed { law_id: String },
    LawEnacted { law_id: String },
    LawStruckDown { law_id: String },
    ElectionCalled { office: String },
    VoteCast { law_id: String, yea: u32, nay: u32 },

    // Information events
    InfoCreated { info_id: String },
    InfoPublished { info_id: String },
    ScandalBroke { info_id: String },

    // Dice
    DiceRolled { skill: String, roll: u32, dc: u32, success: bool },

    // Crisis
    CrisisStarted { description: String },
    CrisisResolved { outcome: String },

    // System
    SaveCompleted,
    Error { message: String },
}

/// Event bus: simple channel-based event system
pub struct EventBus {
    pub sender: crossbeam_channel::Sender<GameEvent>,
    pub receiver: crossbeam_channel::Receiver<GameEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self { sender, receiver }
    }

    pub fn send(&self, event: GameEvent) {
        let _ = self.sender.send(event);
    }

    pub fn drain(&self) -> Vec<GameEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            events.push(event);
        }
        events
    }
}
