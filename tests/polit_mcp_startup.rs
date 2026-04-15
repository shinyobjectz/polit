use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};
use tempfile::TempDir;

#[test]
fn mcp_drives_first_launch_setup_into_title_screen() {
    let server_binary = std::env::var("CARGO_BIN_EXE_polit_mcp").expect("polit_mcp binary path");
    let polit_binary = std::env::var("CARGO_BIN_EXE_polit").expect("polit binary path");
    let home = TempDir::new().unwrap();
    let fake_codex_bin = home.path().join("fake-bin");
    install_fake_codex(&fake_codex_bin);

    let path_env = join_path(&fake_codex_bin);
    let mut client = McpClient::spawn(&server_binary);

    let launch = client.request_ok(
        1,
        "launch",
        json!({
            "binaryPath": polit_binary,
            "home": home.path(),
            "pathEnv": path_env,
            "terminal": {
                "width": 100,
                "height": 30
            }
        }),
    );
    assert_eq!(launch["status"], "ok");
    assert!(launch["lines"]
        .as_array()
        .unwrap()
        .iter()
        .any(|line| line.as_str().unwrap().contains("AI Setup")));

    let send = client.request_ok(
        2,
        "send_keys",
        json!({
            "keys": ["enter"],
            "settleMs": 750,
            "maxLines": 20
        }),
    );
    assert_eq!(send["status"], "ok");

    let wait = client.request_ok(
        3,
        "wait_for_text",
        json!({
            "text": "New Campaign",
            "timeoutMs": 2500,
            "maxLines": 20
        }),
    );
    assert_eq!(wait["status"], "ok");
    assert_eq!(wait["found"], true);

    let config = client.request_ok(
        4,
        "read_file_excerpt",
        json!({
            "path": ".polit/config/ai.toml",
            "maxLines": 10
        }),
    );
    assert!(config["lines"]
        .as_array()
        .unwrap()
        .iter()
        .any(|line| line.as_str().unwrap().contains("provider = \"codex\"")));

    let terminate = client.request_ok(5, "terminate", json!({}));
    assert_eq!(terminate["status"], "terminated");
    client.finish();
}

fn install_fake_codex(bin_dir: &Path) {
    fs::create_dir_all(bin_dir).unwrap();
    let binary_path = if cfg!(windows) {
        bin_dir.join("codex.cmd")
    } else {
        bin_dir.join("codex")
    };

    let script = if cfg!(windows) {
        "@echo off\r\nif \"%1\"==\"login\" if \"%2\"==\"status\" (\r\n  echo Logged in using ChatGPT\r\n  exit /b 0\r\n)\r\necho unexpected codex args %* 1>&2\r\nexit /b 1\r\n"
    } else {
        "#!/bin/sh\nif [ \"$1\" = \"login\" ] && [ \"$2\" = \"status\" ]; then\n  echo \"Logged in using ChatGPT\"\n  exit 0\nfi\necho \"unexpected codex args: $*\" >&2\nexit 1\n"
    };
    fs::write(&binary_path, script).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&binary_path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&binary_path, permissions).unwrap();
    }
}

fn join_path(prefix: &Path) -> String {
    let existing = std::env::var("PATH").unwrap_or_default();
    let separator = if cfg!(windows) { ";" } else { ":" };
    if existing.is_empty() {
        prefix.display().to_string()
    } else {
        format!("{}{}{}", prefix.display(), separator, existing)
    }
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
