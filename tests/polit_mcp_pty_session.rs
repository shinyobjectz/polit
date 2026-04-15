use std::time::Duration;

use polit::mcp::pty_session::{PtySession, PtySessionConfig};
use tempfile::TempDir;

#[test]
fn pty_session_launches_real_binary_and_updates_live_screen() {
    let binary = std::env::var("CARGO_BIN_EXE_polit").expect("polit binary path");
    let home = TempDir::new().unwrap();

    let mut session = PtySession::launch(
        binary,
        PtySessionConfig::new(home.path(), 100, 30),
    )
    .unwrap();

    session
        .wait_for_text("AI Setup", Duration::from_secs(2))
        .unwrap();
    assert!(session
        .screen_lines()
        .iter()
        .any(|line| line.contains("AI Setup")));

    let initial_revision = session.screen_revision();

    session
        .send_key("down", Duration::from_millis(750))
        .unwrap();
    session
        .send_key("enter", Duration::from_millis(750))
        .unwrap();
    session
        .wait_for_text("OpenRouter Key", Duration::from_secs(2))
        .unwrap();

    assert!(session.screen_revision() > initial_revision);
    assert!(session
        .screen_lines()
        .iter()
        .any(|line| line.contains("OpenRouter Key")));

    session.terminate().unwrap();
}
