use std::thread;
use std::time::Duration;

// Integration tests for the core game loop

#[test]
fn test_game_state_creation() {
    let state = polit::engine::GameState::new().unwrap();

    assert_eq!(state.week, 1);
    assert_eq!(state.year, 2024);
    assert_eq!(state.ap_current(), state.config.balance.action_points.local);
    assert_eq!(state.phase, polit::engine::GamePhase::TitleScreen);
}

#[test]
fn test_ap_spending() {
    let mut state = polit::engine::GameState::new().unwrap();

    let initial = state.ap_current();
    state.spend_ap(2);
    assert_eq!(state.ap_current(), initial - 2);

    // Can't go below 0
    state.spend_ap(100);
    assert_eq!(state.ap_current(), 0);
}

#[test]
fn test_ap_reset() {
    let mut state = polit::engine::GameState::new().unwrap();

    state.spend_ap(3);
    state.reset_ap(8);
    assert_eq!(state.ap_current(), 8);
    assert_eq!(state.ap_max(), 8);
}

#[test]
fn test_config_driven_ap() {
    let state = polit::engine::GameState::new().unwrap();

    // AP should match config
    assert_eq!(state.ap_current(), state.config.balance.action_points.local);
    assert_eq!(state.config.balance.action_costs.meet_in_person, 2);
    assert_eq!(state.config.balance.action_costs.phone_call, 1);
    assert_eq!(state.config.balance.action_costs.speech, 1);
}

#[test]
fn test_channel_communication() {
    use polit::engine::channels::*;

    let channels = Channels::new();
    let (ui, game) = channels.split();

    // UI → Game
    ui.send(UiCommand::PlayerInput("hello".into()));
    let cmd = game.try_recv().unwrap();
    match cmd {
        UiCommand::PlayerInput(text) => assert_eq!(text, "hello"),
        _ => panic!("Wrong command type"),
    }

    // Game → UI
    game.send(UiMessage::Narrate("test narration".into()));
    let msgs = ui.drain_messages();
    assert_eq!(msgs.len(), 1);
    match &msgs[0] {
        UiMessage::Narrate(text) => assert_eq!(text, "test narration"),
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_channel_status_update() {
    use polit::engine::channels::*;

    let channels = Channels::new();
    let (ui, game) = channels.split();

    game.send(UiMessage::StatusUpdate {
        week: 5,
        year: 2025,
        phase: "Action".into(),
        ap_current: 3,
        ap_max: 7,
    });

    let msgs = ui.drain_messages();
    assert_eq!(msgs.len(), 1);
    match &msgs[0] {
        UiMessage::StatusUpdate {
            week,
            year,
            phase,
            ap_current,
            ap_max,
        } => {
            assert_eq!(*week, 5);
            assert_eq!(*year, 2025);
            assert_eq!(phase, "Action");
            assert_eq!(*ap_current, 3);
            assert_eq!(*ap_max, 7);
        }
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_threaded_game_loop() {
    use polit::engine::channels::*;

    let state = polit::engine::GameState::new().unwrap();

    let channels = Channels::new();
    let (ui, game_ch) = channels.split();

    let handle = polit::engine::game_thread::spawn_game_thread(state, game_ch);

    // Wait for welcome messages
    thread::sleep(Duration::from_millis(200));
    let msgs = ui.drain_messages();
    assert!(
        msgs.len() >= 3,
        "Expected welcome + briefing messages, got {}",
        msgs.len()
    );

    // Send end turn
    ui.send(UiCommand::EndTurn);
    thread::sleep(Duration::from_millis(200));

    // Should get dusk + dawn messages
    let msgs = ui.drain_messages();
    assert!(!msgs.is_empty(), "Expected turn cycle messages");

    // Quit
    ui.send(UiCommand::Quit);
    handle.join().unwrap();
}

#[test]
fn test_ai_integration_in_game() {
    use polit::engine::channels::*;

    let state = polit::engine::GameState::new().unwrap();

    let channels = Channels::new();
    let (ui, game_ch) = channels.split();

    let handle = polit::engine::game_thread::spawn_game_thread(state, game_ch);

    // Wait for startup
    thread::sleep(Duration::from_millis(200));
    ui.drain_messages();

    // Send player input — should get AI-generated response
    ui.send(UiCommand::PlayerInput(
        "I want to talk about zoning reform".into(),
    ));
    thread::sleep(Duration::from_millis(200));

    let msgs = ui.drain_messages();
    // Should contain narration from mock AI + possible tool call results
    let has_narration = msgs.iter().any(|m| matches!(m, UiMessage::Narrate(_)));
    assert!(has_narration, "Expected AI narration response");

    ui.send(UiCommand::Quit);
    handle.join().unwrap();
}

#[test]
fn test_meet_command_costs_ap() {
    use polit::engine::channels::*;

    let state = polit::engine::GameState::new().unwrap();

    let channels = Channels::new();
    let (ui, game_ch) = channels.split();

    let handle = polit::engine::game_thread::spawn_game_thread(state, game_ch);

    thread::sleep(Duration::from_millis(200));
    ui.drain_messages();

    // Meet costs 2 AP
    ui.send(UiCommand::SlashCommand {
        cmd: "meet".into(),
        args: vec!["Davis".into()],
    });
    thread::sleep(Duration::from_millis(200));

    let msgs = ui.drain_messages();
    // Should have status update with reduced AP
    let status = msgs.iter().find_map(|m| match m {
        UiMessage::StatusUpdate { ap_current, .. } => Some(*ap_current),
        _ => None,
    });
    assert!(status.is_some(), "Expected status update after /meet");
    // AP should be reduced by 2 from initial (5 - 2 = 3)
    assert_eq!(status.unwrap(), 3);

    // Should also have NPC dialogue
    let has_dialogue = msgs
        .iter()
        .any(|m| matches!(m, UiMessage::NpcDialogue { .. }));
    assert!(has_dialogue, "Expected NPC dialogue after /meet");

    ui.send(UiCommand::Quit);
    handle.join().unwrap();
}
