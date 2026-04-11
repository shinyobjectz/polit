use super::{GamePhase, GameState};

/// Advance the game state by one full turn
pub fn tick(state: &mut GameState) {
    // Dawn phase: world simulation advances
    state.phase = GamePhase::Dawn;
    run_dawn(state);

    // Action phase: player spends AP
    state.phase = GamePhase::Action;

    // Dusk phase: resolve consequences
    state.phase = GamePhase::Dusk;
    run_dusk(state);

    // Advance time
    state.week += 1;
    if state.week > 52 {
        state.week = 1;
        state.year += 1;
    }

    state.phase = GamePhase::Downtime;
}

fn run_dawn(state: &mut GameState) {
    // Run simulation systems
    state.schedule.run(&mut state.world);
}

fn run_dusk(_state: &mut GameState) {
    // Resolve consequences — file-based autosave handled by game_thread
}
