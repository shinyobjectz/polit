use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use polit::devtools::frame_dump::buffer_to_text_lines;
use polit::devtools::harness::ScriptedEventSource;
use polit::ui::{run_startup_gate, StartupGateOutcome};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tempfile::tempdir;

#[test]
fn poldev_in_process_startup_enters_real_setup_screen_before_launch() {
    let dir = tempdir().unwrap();
    let ai_config_path = dir.path().join("ai.toml");
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut events = ScriptedEventSource::new(vec![Event::Key(KeyEvent::new(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
    ))]);

    let outcome = run_startup_gate(&mut terminal, &mut events, &ai_config_path).unwrap();

    assert_eq!(outcome, StartupGateOutcome::Cancelled);

    let lines = buffer_to_text_lines(terminal.backend().buffer());
    assert!(lines.iter().any(|line| line.contains("AI Setup")));
    assert!(lines
        .iter()
        .any(|line| line.contains("Choose how POLIT should handle all AI interactions")));
}
