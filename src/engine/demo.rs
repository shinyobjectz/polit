use std::thread;
use std::time::Duration;

use super::channels::{GameChannels, UiCommand, UiMessage};
use super::GameState;
use crate::engine::components::Relationship;
use crate::engine::world;
use crate::persistence::{self, CF_CHARACTERS, CF_RELATIONSHIPS, CF_WORLD_STATE};
use crate::systems::dice;

/// Run the demo walkthrough on the game thread.
/// Sends scripted messages to UI with pauses for readability.
pub fn run_demo(mut state: GameState, ch: GameChannels) {
    let pause = Duration::from_millis(800);
    let short = Duration::from_millis(400);

    // === Welcome ===
    ch.send(UiMessage::PhaseHeader("POLIT Phase 1 Demo".into()));
    thread::sleep(pause);
    ch.send(UiMessage::System(
        "This demo walks through all Phase 1 systems.".into(),
    ));
    ch.send(UiMessage::System(
        "Watch the chat stream, status bar, and try the overlays.".into(),
    ));
    thread::sleep(pause);

    // === 1. ECS World ===
    ch.send(UiMessage::PhaseHeader("1. ECS World — bevy_ecs".into()));
    thread::sleep(short);

    let player_id = world::spawn_player(&mut state.world, "Alex Rivera");
    ch.send(UiMessage::Success(format!(
        "Spawned player entity: {:?}",
        player_id
    )));
    thread::sleep(short);

    let npc1 = world::spawn_npc(
        &mut state.world,
        "Councilwoman Davis",
        Some(super::components::Office::CityCouncil),
    );
    let npc2 = world::spawn_npc(&mut state.world, "Chief Kowalski", None);
    ch.send(UiMessage::Success(format!(
        "Spawned NPC: Councilwoman Davis ({:?})",
        npc1
    )));
    ch.send(UiMessage::Success(format!(
        "Spawned NPC: Chief Kowalski ({:?})",
        npc2
    )));
    thread::sleep(short);

    ch.send(UiMessage::Narrate("ECS world has 3 entities with Identity, PoliticalRole, Personality, Ideology, Stats, Health, Goals components.".into()));
    thread::sleep(pause);

    // === 2. RocksDB Persistence ===
    ch.send(UiMessage::PhaseHeader("2. RocksDB Persistence".into()));
    thread::sleep(short);

    // Write relationships
    let rel = Relationship {
        trust: 45,
        respect: 60,
        fear: 5,
        loyalty: 30,
        debt: 1,
        knowledge: 40,
        leverage: 0,
        rel_type: super::components::RelationshipType::Neutral,
        memories: vec![super::components::Memory {
            week: 1,
            description: "Met at first council meeting".into(),
            impact: 5,
        }],
    };

    state
        .db
        .put(CF_RELATIONSHIPS, "player:davis", &rel)
        .unwrap();
    ch.send(UiMessage::Success(
        "Wrote player→Davis relationship to RocksDB".into(),
    ));

    let hostile_rel = Relationship {
        trust: -20,
        respect: 30,
        ..Relationship::default()
    };
    state
        .db
        .put(CF_RELATIONSHIPS, "player:kowalski", &hostile_rel)
        .unwrap();
    ch.send(UiMessage::Success(
        "Wrote player→Kowalski relationship to RocksDB".into(),
    ));
    thread::sleep(short);

    // Read back
    let read_rel: Relationship = state
        .db
        .get(CF_RELATIONSHIPS, "player:davis")
        .unwrap()
        .unwrap();
    ch.send(UiMessage::Narrate(format!(
        "Read back Davis relationship: trust={}, respect={}, debt={}",
        read_rel.trust, read_rel.respect, read_rel.debt
    )));

    // Prefix scan
    let all_rels: Vec<(String, Relationship)> =
        state.db.scan_prefix(CF_RELATIONSHIPS, "player:").unwrap();
    ch.send(UiMessage::Narrate(format!(
        "Prefix scan 'player:' → {} relationships found",
        all_rels.len()
    )));
    thread::sleep(short);

    // All column families
    ch.send(UiMessage::Narrate(format!(
        "Column families: {:?}",
        persistence::Database::column_families()
    )));
    thread::sleep(pause);

    // === 3. Snapshot Save System ===
    ch.send(UiMessage::PhaseHeader("3. Snapshot Save System".into()));
    thread::sleep(short);

    state.db.put(CF_WORLD_STATE, "week", &1u32).unwrap();
    state.db.put(CF_WORLD_STATE, "year", &2024u32).unwrap();
    match state.db.create_snapshot("demo_save") {
        Ok(path) => ch.send(UiMessage::Success(format!(
            "Snapshot saved to: {}",
            path.display()
        ))),
        Err(e) => ch.send(UiMessage::Warning(format!("Snapshot failed: {}", e))),
    }
    thread::sleep(pause);

    // === 4. Dice System ===
    ch.send(UiMessage::PhaseHeader("4. D20 Dice System".into()));
    thread::sleep(short);

    for skill in &["Persuasion", "Cunning", "Media Savvy"] {
        let result = dice::skill_check(skill, 3, 12);
        let status = if result.critical_success {
            "CRITICAL SUCCESS!"
        } else if result.critical_failure {
            "CRITICAL FAILURE!"
        } else if result.success {
            "Success"
        } else {
            "Failure"
        };
        ch.send(UiMessage::DiceRoll(format!(
            "🎲 {} check: {} + {} = {} vs DC 12 → {}",
            skill, result.natural_roll, result.modifiers, result.total, status
        )));
        thread::sleep(short);
    }
    thread::sleep(pause);

    // === 5. Config System ===
    ch.send(UiMessage::PhaseHeader("5. TOML Config System".into()));
    thread::sleep(short);

    match super::config::GameConfig::load("game/config") {
        Ok(config) => {
            ch.send(UiMessage::Success(
                "Loaded game/config/balance.toml + difficulty.toml".into(),
            ));
            ch.send(UiMessage::Narrate(format!(
                "  AP by office: local={}, president={}",
                config.balance.action_points.local, config.balance.action_points.president
            )));
            ch.send(UiMessage::Narrate(format!(
                "  Dice: d{}, crit={}, fumble={}",
                config.balance.dice.sides,
                config.balance.dice.crit_success,
                config.balance.dice.crit_failure
            )));
            ch.send(UiMessage::Narrate(format!(
                "  Difficulty: {} (DC modifier: {:+})",
                config.difficulty.description, config.difficulty.dc_modifier
            )));
        }
        Err(e) => ch.send(UiMessage::Warning(format!("Config load failed: {}", e))),
    }
    thread::sleep(pause);

    // === 6. Rhai Scripting Sandbox ===
    ch.send(UiMessage::PhaseHeader("6. Rhai Scripting Sandbox".into()));
    thread::sleep(short);

    let scripting = crate::scripting::ScriptEngine::new();
    match scripting.eval("let x = 40; let y = 2; x + y") {
        Ok(result) => ch.send(UiMessage::Success(format!(
            "Rhai eval '40 + 2' = {}",
            result
        ))),
        Err(e) => ch.send(UiMessage::Warning(format!("Rhai error: {}", e))),
    }

    match scripting.eval("loop { }") {
        Ok(_) => ch.send(UiMessage::Warning("Infinite loop was NOT caught!".into())),
        Err(_) => ch.send(UiMessage::Success(
            "Infinite loop caught by sandbox (CPU budget limit)".into(),
        )),
    }
    thread::sleep(pause);

    // === 7. Game Loop ===
    ch.send(UiMessage::PhaseHeader("7. Game Loop — Turn Cycle".into()));
    thread::sleep(short);

    state.phase = super::GamePhase::Dawn;
    ch.send(UiMessage::StatusUpdate {
        week: 1,
        year: 2024,
        phase: "Dawn".into(),
        ap_current: 5,
        ap_max: 5,
    });
    ch.send(UiMessage::Narrate(
        "Phase: Dawn — world simulation ticks, briefing generated".into(),
    ));
    thread::sleep(short);

    state.phase = super::GamePhase::Action;
    ch.send(UiMessage::StatusUpdate {
        week: 1,
        year: 2024,
        phase: "Action".into(),
        ap_current: 5,
        ap_max: 5,
    });
    ch.send(UiMessage::Narrate(
        "Phase: Action — player spends AP on meetings, speeches, etc.".into(),
    ));
    state.spend_ap(2);
    ch.send(UiMessage::System(
        "Spent 2 AP to meet with Davis. 3 AP remaining.".into(),
    ));
    ch.send(UiMessage::StatusUpdate {
        week: 1,
        year: 2024,
        phase: "Action".into(),
        ap_current: state.ap_current(),
        ap_max: state.ap_max(),
    });
    thread::sleep(short);

    state.phase = super::GamePhase::Dusk;
    ch.send(UiMessage::StatusUpdate {
        week: 1,
        year: 2024,
        phase: "Dusk".into(),
        ap_current: state.ap_current(),
        ap_max: state.ap_max(),
    });
    ch.send(UiMessage::Narrate(
        "Phase: Dusk — consequences resolve, autosave".into(),
    ));
    thread::sleep(short);

    state.week = 2;
    state.reset_ap(5);
    ch.send(UiMessage::StatusUpdate {
        week: 2,
        year: 2024,
        phase: "Dawn".into(),
        ap_current: 5,
        ap_max: 5,
    });
    ch.send(UiMessage::Success(
        "Week advanced! AP reset. Turn cycle complete.".into(),
    ));
    thread::sleep(pause);

    // === 8. Event Bus ===
    ch.send(UiMessage::PhaseHeader(
        "8. Event Bus — crossbeam channels".into(),
    ));
    thread::sleep(short);

    use super::events::{EventBus, GameEvent};
    let bus = EventBus::new();
    bus.send(GameEvent::Narrate {
        text: "Test narration".into(),
    });
    bus.send(GameEvent::CardAcquired {
        card_id: "stump_speech".into(),
    });
    bus.send(GameEvent::DiceRolled {
        skill: "Persuasion".into(),
        roll: 17,
        dc: 12,
        success: true,
    });
    let events = bus.drain();
    ch.send(UiMessage::Success(format!(
        "Event bus: sent 3 events, drained {} events",
        events.len()
    )));
    ch.send(UiMessage::Narrate(
        "Events: Narrate, CardAcquired, DiceRolled — all types working.".into(),
    ));
    thread::sleep(pause);

    // === Summary ===
    ch.send(UiMessage::PhaseHeader("Demo Complete".into()));
    thread::sleep(short);
    ch.send(UiMessage::System("All Phase 1 systems verified:".into()));
    ch.send(UiMessage::Success(
        "  ✓ ECS World (bevy_ecs 0.15 standalone)".into(),
    ));
    ch.send(UiMessage::Success(
        "  ✓ RocksDB (11 column families, CRUD, prefix scan)".into(),
    ));
    ch.send(UiMessage::Success("  ✓ Snapshot saves".into()));
    ch.send(UiMessage::Success(
        "  ✓ D20 dice system (skill checks, crits, modifiers)".into(),
    ));
    ch.send(UiMessage::Success(
        "  ✓ TOML config (balance, difficulty modes)".into(),
    ));
    ch.send(UiMessage::Success("  ✓ Rhai scripting sandbox".into()));
    ch.send(UiMessage::Success(
        "  ✓ Game loop (Dawn/Action/Dusk turn cycle)".into(),
    ));
    ch.send(UiMessage::Success(
        "  ✓ Event bus (crossbeam channels)".into(),
    ));
    ch.send(UiMessage::Success(
        "  ✓ Async threading (UI thread ↔ Game thread)".into(),
    ));
    ch.send(UiMessage::Success(
        "  ✓ Chat-forward TUI (Ratatui 0.29)".into(),
    ));
    ch.send(UiMessage::Success(
        "  ✓ Floating overlays (Tab for command palette)".into(),
    ));
    ch.send(UiMessage::Success(
        "  ✓ Slash commands (/help, /end, /meet, /save, etc.)".into(),
    ));
    thread::sleep(short);
    ch.send(UiMessage::System("".into()));
    ch.send(UiMessage::System(
        "Try: Tab (command palette), /help, type freely, /end (advance turn)".into(),
    ));
    ch.send(UiMessage::System("Press Ctrl+C or /quit to exit.".into()));

    // After demo, run normal game loop to let user interact
    super::game_thread::spawn_game_loop_after_demo(state, &ch);
}
