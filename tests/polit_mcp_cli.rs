use std::io::{BufRead, BufReader, Read, Write};
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
    assert_eq!(response["result"]["status"], "ok");
    assert_eq!(response["result"]["sessionActive"], true);
}

#[test]
fn initialize_and_list_tools_over_framed_mcp_stdio() {
    let binary = std::env::var("CARGO_BIN_EXE_polit_mcp")
        .expect("cargo should expose the polit_mcp test binary");

    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("polit_mcp should start");

    let mut stdin = child.stdin.take().expect("stdin should be piped");
    let stdout = child.stdout.take().expect("stdout should be piped");
    let mut reader = BufReader::new(stdout);

    write_framed(
        &mut stdin,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {
                    "name": "codex-test",
                    "version": "0"
                }
            }
        }),
    );

    let initialize = read_framed(&mut reader);
    assert_eq!(initialize["jsonrpc"], "2.0");
    assert_eq!(initialize["id"], 1);
    assert_eq!(initialize["result"]["serverInfo"]["name"], "polit");
    assert!(initialize["result"]["capabilities"]["tools"].is_object());

    write_framed(
        &mut stdin,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
    );

    write_framed(
        &mut stdin,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
    );

    let tools = read_framed(&mut reader);
    assert_eq!(tools["id"], 2);
    let tool_list = tools["result"]["tools"]
        .as_array()
        .expect("tools/list should return an array");
    assert!(
        tool_list.iter().any(|tool| tool["name"] == "launch"),
        "expected launch tool in {tool_list:?}"
    );
    assert!(
        tool_list.iter().any(|tool| tool["name"] == "send_keys"),
        "expected send_keys tool in {tool_list:?}"
    );

    drop(stdin);
    let output = child.wait_with_output().expect("polit_mcp should exit");
    assert!(
        output.status.success(),
        "polit_mcp exited unsuccessfully: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_framed(stdin: &mut impl Write, value: &serde_json::Value) {
    let payload = serde_json::to_vec(value).expect("request payload should serialize");
    write!(stdin, "Content-Length: {}\r\n\r\n", payload.len()).expect("header should write");
    stdin.write_all(&payload).expect("payload should write");
    stdin.flush().expect("request should flush");
}

fn read_framed(reader: &mut impl BufRead) -> serde_json::Value {
    let mut content_length = None;

    loop {
        let mut line = String::new();
        reader.read_line(&mut line).expect("header line should read");
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }

        if let Some((name, value)) = trimmed.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                content_length = Some(
                    value
                        .trim()
                        .parse::<usize>()
                        .expect("content length should parse"),
                );
            }
        }
    }

    let content_length = content_length.expect("response should include Content-Length");
    let mut payload = vec![0u8; content_length];
    reader
        .read_exact(&mut payload)
        .expect("payload should read");
    serde_json::from_slice(&payload).expect("payload should be valid json")
}
