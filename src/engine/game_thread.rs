use std::thread;

use super::channels::{GameChannels, UiCommand, UiMessage};
use super::{GamePhase, GameState};
use crate::systems::dice;

/// Run the game logic on a dedicated thread
pub fn spawn_game_thread(mut state: GameState, channels: GameChannels) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("polit-game".to_string())
        .spawn(move || {
            game_loop_main(&mut state, &channels);
        })
        .expect("Failed to spawn game thread")
}

/// After the demo finishes, run the normal interactive game loop
pub fn spawn_game_loop_after_demo(mut state: GameState, ch: &GameChannels) {
    // The demo already set up week 2, so just enter the interactive loop
    state.phase = GamePhase::Action;
    send_status(&state, ch);

    loop {
        match ch.try_recv() {
            Some(UiCommand::Quit) => {
                ch.send(UiMessage::Shutdown);
                break;
            }
            Some(UiCommand::EndTurn) => {
                run_dusk(&mut state, ch);
                advance_week(&mut state);
                send_status(&state, ch);
                run_dawn(&mut state, ch);
            }
            Some(UiCommand::PlayerInput(text)) => {
                handle_player_input(&mut state, ch, &text);
            }
            Some(UiCommand::SlashCommand { cmd, args }) => {
                handle_slash_command(&mut state, ch, &cmd, &args);
            }
            Some(UiCommand::SaveGame(name)) => match state.db.create_snapshot(&name) {
                Ok(path) => ch.send(UiMessage::Success(format!("Saved: {}", path.display()))),
                Err(e) => ch.send(UiMessage::Warning(format!("Save failed: {}", e))),
            },
            Some(UiCommand::LoadGame(name)) => {
                ch.send(UiMessage::System(format!("Loading '{}' (Phase 2)", name)));
            }
            None => {
                thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}

fn game_loop_main(state: &mut GameState, ch: &GameChannels) {
    // Send initial welcome
    ch.send(UiMessage::System(
        "Welcome to POLIT — The American Politics Simulator".into(),
    ));
    ch.send(UiMessage::System(
        "Type /help for commands, or just start typing.".into(),
    ));
    send_status(state, ch);

    // Start the first week
    run_dawn(state, ch);

    loop {
        // Process UI commands (non-blocking)
        match ch.try_recv() {
            Some(UiCommand::Quit) => {
                ch.send(UiMessage::Shutdown);
                break;
            }
            Some(UiCommand::EndTurn) => {
                run_dusk(state, ch);
                advance_week(state);
                send_status(state, ch);
                run_dawn(state, ch);
            }
            Some(UiCommand::PlayerInput(text)) => {
                handle_player_input(state, ch, &text);
            }
            Some(UiCommand::SlashCommand { cmd, args }) => {
                handle_slash_command(state, ch, &cmd, &args);
            }
            Some(UiCommand::SaveGame(name)) => match state.db.create_snapshot(&name) {
                Ok(path) => ch.send(UiMessage::Success(format!(
                    "Game saved: {}",
                    path.display()
                ))),
                Err(e) => ch.send(UiMessage::Warning(format!("Save failed: {}", e))),
            },
            Some(UiCommand::LoadGame(name)) => {
                ch.send(UiMessage::System(format!(
                    "Loading save '{}' (not yet implemented)",
                    name
                )));
            }
            None => {
                // No commands, sleep briefly to not spin
                thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}

fn run_dawn(state: &mut GameState, ch: &GameChannels) {
    state.phase = GamePhase::Dawn;
    send_status(state, ch);

    ch.send(UiMessage::PhaseHeader(format!(
        "Week {}, {} Begins",
        state.week, state.year
    )));

    // Morning briefing (mock for now — Phase 2 will use AI)
    ch.send(UiMessage::Narrate("■ MORNING BRIEFING".into()));
    ch.send(UiMessage::Narrate(format!(
        "It's week {} of {}. The political landscape is shifting.",
        state.week, state.year
    )));

    if state.week == 1 {
        ch.send(UiMessage::Narrate(
            "You've just taken office as a city council member. Your constituents \
             are watching to see what kind of leader you'll be."
                .into(),
        ));
    } else {
        // Generate some variety in briefings
        let briefings = [
            "Local business owners are concerned about the new zoning proposal.",
            "A community group is requesting a meeting about park funding.",
            "The local paper ran an editorial questioning your priorities.",
            "Your approval rating ticked up 2 points this week.",
            "A rival council member is building support for an alternative budget.",
            "Construction on the water treatment plant is behind schedule.",
            "A local factory announced 50 new jobs — good news for the district.",
            "School board elections are coming up. Several candidates want your endorsement.",
        ];
        let idx = (state.week as usize * 7 + state.year as usize) % briefings.len();
        ch.send(UiMessage::Narrate(briefings[idx].into()));
    }

    ch.send(UiMessage::Narrate(format!(
        "\nYou have {} Action Points this week. What will you do?",
        state.ap_current()
    )));

    state.phase = GamePhase::Action;
    send_status(state, ch);
}

fn run_dusk(state: &mut GameState, ch: &GameChannels) {
    state.phase = GamePhase::Dusk;
    send_status(state, ch);

    ch.send(UiMessage::System("Resolving consequences...".into()));

    // Autosave
    let save_name = format!("autosave_w{}_y{}", state.week, state.year);
    match state.db.create_snapshot(&save_name) {
        Ok(_) => ch.send(UiMessage::System("Autosaved.".into())),
        Err(e) => ch.send(UiMessage::Warning(format!("Autosave failed: {}", e))),
    }
}

fn advance_week(state: &mut GameState) {
    state.week += 1;
    if state.week > 52 {
        state.week = 1;
        state.year += 1;
    }
    // Reset AP
    let max = state.ap_max();
    state.reset_ap(max);
}

fn handle_player_input(state: &mut GameState, ch: &GameChannels, text: &str) {
    if state.phase != GamePhase::Action {
        ch.send(UiMessage::Warning(
            "You can only act during the Action phase.".into(),
        ));
        return;
    }

    // Echo player input
    ch.send(UiMessage::Narrate(format!("> {}", text)));

    // Mock AI response (will route through ONNX provider when model loaded)
    ch.send(UiMessage::Narrate(
        "The room considers your words carefully. (AI responses coming in Phase 2)".into(),
    ));

    // Example dice roll
    let result = dice::skill_check("Persuasion", 3, 12);
    ch.send(UiMessage::DiceRoll(format!(
        "🎲 Persuasion check: rolled {} + {} = {} vs DC {} → {}",
        result.natural_roll,
        result.modifiers,
        result.total,
        result.dc,
        if result.critical_success {
            "CRITICAL SUCCESS!"
        } else if result.critical_failure {
            "CRITICAL FAILURE!"
        } else if result.success {
            "Success"
        } else {
            "Failure"
        }
    )));

    // Spend 1 AP
    state.spend_ap(1);
    if state.ap_current() <= 0 {
        ch.send(UiMessage::System(
            "You've used all your Action Points for this week.".into(),
        ));
        ch.send(UiMessage::System(
            "Type /end to advance to next week.".into(),
        ));
    }
    send_status(state, ch);
}

fn handle_slash_command(state: &mut GameState, ch: &GameChannels, cmd: &str, args: &[String]) {
    match cmd {
        "meet" => {
            if args.is_empty() {
                ch.send(UiMessage::System("Usage: /meet <npc name>".into()));
            } else {
                let name = args.join(" ");
                if state.ap_current() < 2 {
                    ch.send(UiMessage::Warning(
                        "Not enough AP. Meetings cost 2 AP.".into(),
                    ));
                } else {
                    state.spend_ap(2);
                    ch.send(UiMessage::System(format!(
                        "You spend 2 AP to meet with {}.",
                        name
                    )));
                    ch.send(UiMessage::NpcDialogue {
                        name: name.clone(),
                        text: format!(
                            "\"Thanks for making time, {}. I wanted to discuss something with you.\"\n\
                             (Full NPC conversations coming in Phase 2)",
                            "Councilmember"
                        ),
                    });
                    send_status(state, ch);
                }
            }
        }
        "draft" => {
            ch.send(UiMessage::System(
                "Entering law drafting mode... (Coming in Phase 6)".into(),
            ));
        }
        "speech" => {
            if state.ap_current() < 1 {
                ch.send(UiMessage::Warning(
                    "Not enough AP. Speeches cost 1 AP.".into(),
                ));
            } else {
                state.spend_ap(1);
                ch.send(UiMessage::System("You step to the podium...".into()));
                ch.send(UiMessage::Narrate(
                    "The small crowd at city hall turns to listen. \
                     (Speech mechanics coming in Phase 9)"
                        .into(),
                ));
                send_status(state, ch);
            }
        }
        "save" => {
            let name = if args.is_empty() {
                format!("manual_w{}_y{}", state.week, state.year)
            } else {
                args.join("_")
            };
            match state.db.create_snapshot(&name) {
                Ok(path) => ch.send(UiMessage::Success(format!("Saved: {}", path.display()))),
                Err(e) => ch.send(UiMessage::Warning(format!("Save failed: {}", e))),
            }
        }
        "load" => {
            if args.is_empty() {
                ch.send(UiMessage::System("Usage: /load <save_name>".into()));
            } else {
                ch.send(UiMessage::System(format!(
                    "Loading '{}' (coming soon)",
                    args.join(" ")
                )));
            }
        }
        _ => {
            ch.send(UiMessage::System(format!("Unknown command: /{}", cmd)));
        }
    }
}

fn send_status(state: &GameState, ch: &GameChannels) {
    let phase_str = match state.phase {
        GamePhase::TitleScreen => "Title",
        GamePhase::Dawn => "Dawn",
        GamePhase::Action => "Action",
        GamePhase::Dusk => "Dusk",
        GamePhase::Downtime => "Downtime",
        GamePhase::Event(_) => "Event",
        GamePhase::CharacterCreation => "Create",
        GamePhase::ElectionNight => "Election",
        GamePhase::CareerEnd => "End",
    };

    ch.send(UiMessage::StatusUpdate {
        week: state.week,
        year: state.year,
        phase: phase_str.to_string(),
        ap_current: state.ap_current(),
        ap_max: state.ap_max(),
    });
}
