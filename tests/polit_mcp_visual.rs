use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};
use tempfile::TempDir;

#[test]
fn mcp_resize_and_screenshot_create_real_terminal_artifacts() {
    let server_binary = std::env::var("CARGO_BIN_EXE_polit_mcp").expect("polit_mcp binary path");
    let polit_binary = std::env::var("CARGO_BIN_EXE_polit").expect("polit binary path");
    let home = TempDir::new().unwrap();

    let mut client = McpClient::spawn(&server_binary);
    let launch = client.request(
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

    let resize = client.request(
        2,
        "resize",
        json!({
            "width": 80,
            "height": 24,
            "maxLines": 10
        }),
    );
    assert_eq!(resize["status"], "ok");
    assert_eq!(resize["terminal"]["width"], 80);
    assert_eq!(resize["terminal"]["height"], 24);

    let screenshot = client.request(
        3,
        "screenshot",
        json!({
            "label": "startup"
        }),
    );
    assert_eq!(screenshot["status"], "ok");
    let artifact = PathBuf::from(screenshot["artifactPath"].as_str().unwrap());
    assert!(artifact.exists(), "expected screenshot artifact at {}", artifact.display());

    let terminate = client.request(4, "terminate", json!({}));
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

    fn request(&mut self, id: u64, method: &str, params: Value) -> Value {
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
