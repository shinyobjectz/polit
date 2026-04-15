use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};
use tempfile::TempDir;

#[test]
fn mcp_observability_tools_are_bounded_and_reject_escaped_paths() {
    let server_binary = std::env::var("CARGO_BIN_EXE_polit_mcp").expect("polit_mcp binary path");
    let polit_binary = std::env::var("CARGO_BIN_EXE_polit").expect("polit binary path");
    let home = TempDir::new().unwrap();

    fs::create_dir_all(home.path().join(".polit/saves/campaign-1")).unwrap();
    fs::create_dir_all(home.path().join(".polit/logs")).unwrap();
    fs::create_dir_all(home.path().join(".polit/config")).unwrap();
    fs::write(
        home.path().join(".polit/saves/campaign-1/meta.toml"),
        "name = \"Test Campaign\"\nrole = \"Mayor\"\n",
    )
    .unwrap();
    fs::write(
        home.path().join(".polit/logs/runtime.log"),
        "line one\nline two\nline three\n",
    )
    .unwrap();
    fs::write(
        home.path().join(".polit/config/ai.toml"),
        "provider = \"codex\"\nmodel = \"gpt-5-codex\"\n",
    )
    .unwrap();

    let mut client = McpClient::spawn(&server_binary);
    let launch = client.request_ok(
        1,
        "launch",
        json!({
            "binaryPath": polit_binary,
            "home": home.path(),
            "terminal": {
                "width": 100,
                "height": 30
            }
        }),
    );
    assert_eq!(launch["status"], "ok");

    let saves = client.request_ok(2, "read_save_metadata", json!({ "maxEntries": 5 }));
    assert_eq!(saves["status"], "ok");
    assert!(saves["saveEntries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["path"].as_str().unwrap().contains("campaign-1/meta.toml")));

    let logs = client.request_ok(
        3,
        "read_recent_logs",
        json!({
            "logKind": "runtime",
            "maxLines": 2
        }),
    );
    assert_eq!(logs["status"], "ok");
    assert_eq!(logs["lines"], json!(["line two", "line three"]));

    let excerpt = client.request_ok(
        4,
        "read_file_excerpt",
        json!({
            "path": ".polit/config/ai.toml",
            "maxLines": 4
        }),
    );
    assert_eq!(excerpt["status"], "ok");
    assert!(excerpt["lines"]
        .as_array()
        .unwrap()
        .iter()
        .any(|line| line.as_str().unwrap().contains("provider = \"codex\"")));

    let rejected = client.request_response(
        5,
        "read_file_excerpt",
        json!({
            "path": "../../etc/passwd",
            "maxLines": 4
        }),
    );
    assert!(rejected["error"].is_object(), "expected an error response: {rejected}");

    let terminate = client.request_ok(6, "terminate", json!({}));
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
        let response = self.request_response(id, method, params);
        assert!(
            response["error"].is_null(),
            "unexpected error response: {response}"
        );
        response["result"].clone()
    }

    fn request_response(&mut self, id: u64, method: &str, params: Value) -> Value {
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
        assert_eq!(response["id"], id);
        response
    }

    fn finish(mut self) {
        self.stdin.take();
        let status = self.child.wait().expect("polit_mcp should exit cleanly");
        assert!(status.success(), "polit_mcp exited with {status}");
    }
}
