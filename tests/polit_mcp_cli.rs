use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[test]
fn launch_request_is_recognized_by_polit_mcp_stdio_server() {
    let binary = std::env::var("CARGO_BIN_EXE_polit_mcp")
        .expect("cargo should expose the polit_mcp test binary");

    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("polit_mcp should start");

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "launch",
        "params": {
            "terminal": {
                "width": 100,
                "height": 30
            }
        }
    });

    let mut stdin = child.stdin.take().expect("stdin should be piped");
    writeln!(stdin, "{request}").expect("request should write");
    drop(stdin);

    let stdout = child.stdout.take().expect("stdout should be piped");
    let mut reader = BufReader::new(stdout);
    let mut response_line = String::new();
    reader
        .read_line(&mut response_line)
        .expect("response should be readable");

    let output = child.wait_with_output().expect("polit_mcp should exit");
    assert!(
        output.status.success(),
        "polit_mcp exited unsuccessfully: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let response: serde_json::Value =
        serde_json::from_str(response_line.trim()).expect("response should be valid json");
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert_eq!(response["result"]["status"], "not_implemented");
    assert_eq!(response["result"]["method"], "launch");
}
