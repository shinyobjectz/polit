pub mod world;
pub mod game_loop;
pub mod game_thread;
pub mod components;
pub mod events;
pub mod channels;
pub mod config;
pub mod demo;

use bevy_ecs::prelude::*;

use crate::persistence::Database;

/// Core game state
pub struct GameState {
    pub world: World,
    pub schedule: Schedule,
    pub week: u32,
    pub year: u32,
    pub phase: GamePhase,
    pub db: Database,
    ap_current: i32,
    ap_max: i32,
}

/// Current phase of the game loop
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamePhase {
    TitleScreen,
    CharacterCreation,
    Dawn,
    Action,
    Event(EventPhaseType),
    Dusk,
    Downtime,
    ElectionNight,
    CareerEnd,
}

/// Types of special event phases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPhaseType {
    Crisis,
    Debate,
    FloorVote,
    PressConference,
    Trial,
    Negotiation,
    Custom,
}

impl GameState {
    pub fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let db = Database::open(db_path)?;
        let mut world = World::new();
        let schedule = Schedule::default();

        world.insert_resource(GameClock { week: 1, year: 2024 });

        Ok(Self {
            world,
            schedule,
            week: 1,
            year: 2024,
            phase: GamePhase::TitleScreen,
            db,
            ap_current: 5,
            ap_max: 5,
        })
    }

    pub fn ap_current(&self) -> i32 {
        self.ap_current
    }

    pub fn ap_max(&self) -> i32 {
        self.ap_max
    }

    pub fn spend_ap(&mut self, amount: i32) {
        self.ap_current = (self.ap_current - amount).max(0);
    }

    pub fn reset_ap(&mut self, max: i32) {
        self.ap_max = max;
        self.ap_current = max;
    }
}

/// Global game clock resource
#[derive(Resource, Debug, Clone)]
pub struct GameClock {
    pub week: u32,
    pub year: u32,
}

/// Run the game without UI (for testing)
pub fn run_headless() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let mut state = GameState::new(dir.path().to_str().unwrap())?;
    state.phase = GamePhase::Dawn;

    tracing::info!("Headless mode: running 1 turn");
    state.phase = GamePhase::Action;
    state.phase = GamePhase::Dusk;
    state.week += 1;
    tracing::info!("Week {} complete", state.week);

    Ok(())
}
