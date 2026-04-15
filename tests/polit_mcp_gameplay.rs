use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};
use tempfile::TempDir;

#[test]
fn mcp_runs_a_mock_gameplay_turn_through_the_real_binary() {
    let server_binary = std::env::var("CARGO_BIN_EXE_polit_mcp").expect("polit_mcp binary path");
    let polit_binary = std::env::var("CARGO_BIN_EXE_polit").expect("polit binary path");
    let home = TempDir::new().unwrap();

    fs::create_dir_all(home.path().join(".polit/config")).unwrap();
    fs::create_dir_all(home.path().join(".polit/saves/current")).unwrap();
    fs::write(
        home.path().join(".polit/config/ai.toml"),
        "provider = \"codex\"\nmodel = \"gpt-5-codex\"\n",
    )
    .unwrap();
    fs::write(
        home.path().join(".polit/saves/current/character.yaml"),
        "name: Tester\nrole: Mayor\n",
    )
    .unwrap();

    let mut client = McpClient::spawn(&server_binary);
    let launch = client.request_ok(
        1,
        "launch",
        json!({
            "binaryPath": polit_binary,
            "home": home.path(),
            "args": ["--mock"],
            "terminal": {
                "width": 100,
                "height": 30
            }
        }),
    );
    assert_eq!(launch["status"], "ok");

    let title = client.request_ok(
        2,
        "wait_for_text",
        json!({
            "text": "Continue Campaign",
            "timeoutMs": 2500,
            "maxLines": 20
        }),
    );
    assert_eq!(title["found"], true);

    let continue_game = client.request_ok(
        3,
        "send_keys",
        json!({
            "keys": ["down", "enter"],
            "settleMs": 1000,
            "maxLines": 30
        }),
    );
    assert_eq!(continue_game["status"], "ok");

    let in_game = client.request_ok(
        4,
        "wait_for_text",
        json!({
            "text": "Week 1, 2024",
            "timeoutMs": 3000,
            "maxLines": 30
        }),
    );
    assert_eq!(in_game["found"], true);

    let end_turn = client.request_ok(
        5,
        "send_keys",
        json!({
            "text": "/end",
            "keys": ["enter"],
            "settleMs": 1000,
            "maxLines": 30
        }),
    );
    assert_eq!(end_turn["status"], "ok");

    let next_week = client.request_ok(
        6,
        "wait_for_text",
        json!({
            "text": "Week 2, 2024",
            "timeoutMs": 4000,
            "maxLines": 30
        }),
    );
    assert_eq!(next_week["found"], true);

    let terminate = client.request_ok(7, "terminate", json!({}));
    assert_eq!(terminate["status"], "terminated");
    client.finish();
}

struct McpClient {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl McpClient {
    fn spawn(binary: &str) -> Self {
        let mut child = Command::new(binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("polit_mcp should start");

        let stdin = child.stdin.take().expect("stdin should be piped");
        let stdout = child.stdout.take().expect("stdout should be piped");

        Self {
            child,
            stdin: Some(stdin),
            stdout: BufReader::new(stdout),
        }
    }

    fn request_ok(&mut self, id: u64, method: &str, params: Value) -> Value {
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let stdin = self.stdin.as_mut().expect("stdin should still be open");
        writeln!(stdin, "{request}").expect("request should write");
        stdin.flush().expect("request should flush");

        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .expect("response should be readable");

        let response: Value = serde_json::from_str(line.trim()).expect("response should be json");
        assert!(
            response["error"].is_null(),
            "unexpected error response: {response}"
        );
        assert_eq!(response["id"], id);
        response["result"].clone()
    }

    fn finish(mut self) {
        self.stdin.take();
        let status = self.child.wait().expect("polit_mcp should exit cleanly");
        assert!(status.success(), "polit_mcp exited with {status}");
    }
}
