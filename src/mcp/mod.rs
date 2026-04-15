pub mod inspect;
pub mod pty_session;
pub mod session;
pub mod tools;

use std::error::Error;
use std::io::{BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use self::session::SessionManager;

pub fn run_stdio_server<R: BufRead, W: Write>(
    mut reader: R,
    mut writer: W,
) -> Result<(), Box<dyn Error>> {
    let mut sessions = SessionManager::default();

    while let Some(message) = read_message(&mut reader)? {
        let response = match serde_json::from_str::<RpcRequest>(&message.payload) {
            Ok(request) => sessions.handle(request),
            Err(error) => Some(RpcResponse::invalid_request(Some(Value::Null), error.to_string())),
        };

        if let Some(response) = response {
            write_response(&mut writer, &message.framing, &response)?;
        }
    }

    Ok(())
}

fn read_message<R: BufRead>(reader: &mut R) -> Result<Option<IncomingMessage>, Box<dyn Error>> {
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return Ok(None);
        }

        if buffer.starts_with(b"Content-Length:") {
            return read_framed_message(reader);
        }

        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        if line.trim().is_empty() {
            continue;
        }

        return Ok(Some(IncomingMessage {
            payload: line,
            framing: ResponseFraming::LineDelimited,
        }));
    }
}

fn read_framed_message<R: BufRead>(reader: &mut R) -> Result<Option<IncomingMessage>, Box<dyn Error>> {
    let mut content_length = None;

    loop {
        let mut header_line = String::new();
        if reader.read_line(&mut header_line)? == 0 {
            return Ok(None);
        }

        let trimmed = header_line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }

        let Some((name, value)) = trimmed.split_once(':') else {
            return Err(format!("invalid MCP header line: {trimmed}").into());
        };

        if name.eq_ignore_ascii_case("Content-Length") {
            content_length = Some(value.trim().parse::<usize>()?);
        }
    }

    let content_length = content_length.ok_or("missing Content-Length header")?;
    let mut payload = vec![0u8; content_length];
    reader.read_exact(&mut payload)?;

    Ok(Some(IncomingMessage {
        payload: String::from_utf8(payload)?,
        framing: ResponseFraming::ContentLength,
    }))
}

fn write_response<W: Write>(
    writer: &mut W,
    framing: &ResponseFraming,
    response: &RpcResponse,
) -> Result<(), Box<dyn Error>> {
    match framing {
        ResponseFraming::LineDelimited => {
            serde_json::to_writer(&mut *writer, response)?;
            writer.write_all(b"\n")?;
        }
        ResponseFraming::ContentLength => {
            let payload = serde_json::to_vec(response)?;
            write!(writer, "Content-Length: {}\r\n\r\n", payload.len())?;
            writer.write_all(&payload)?;
        }
    }
    writer.flush()?;
    Ok(())
}

#[derive(Debug)]
struct IncomingMessage {
    payload: String,
    framing: ResponseFraming,
}

#[derive(Debug, Clone, Copy)]
enum ResponseFraming {
    LineDelimited,
    ContentLength,
}

#[derive(Debug, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct RpcResponse {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

impl RpcResponse {
    pub fn success(id: impl Into<Option<Value>>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id: id.into(),
            result: Some(result),
            error: None,
        }
    }

    pub fn invalid_request(id: impl Into<Option<Value>>, message: String) -> Self {
        Self::error(id, -32600, message)
    }

    pub fn method_not_found(id: impl Into<Option<Value>>, method: &str) -> Self {
        Self::error(id, -32601, format!("unknown method '{method}'"))
    }

    pub fn internal_error(id: impl Into<Option<Value>>, message: String) -> Self {
        Self::error(id, -32603, message)
    }

    fn error(id: impl Into<Option<Value>>, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0",
            id: id.into(),
            result: None,
            error: Some(RpcError { code, message }),
        }
    }
}

pub fn tool_definitions() -> Vec<Value> {
    vec![
        tool_definition(
            "launch",
            "Launch the real compiled polit binary in a live PTY session.",
            json!({
                "type": "object",
                "properties": {
                    "binaryPath": { "type": "string" },
                    "home": { "type": "string" },
                    "pathEnv": { "type": "string" },
                    "args": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "terminal": {
                        "type": "object",
                        "properties": {
                            "width": { "type": "integer", "minimum": 1 },
                            "height": { "type": "integer", "minimum": 1 }
                        }
                    }
                }
            }),
        ),
        tool_definition(
            "send_keys",
            "Send keyboard input to the active polit session. Text is typed before keys are pressed.",
            json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string" },
                    "keys": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "settleMs": { "type": "integer", "minimum": 0 },
                    "maxLines": { "type": "integer", "minimum": 1 }
                }
            }),
        ),
        tool_definition(
            "read_screen",
            "Read a bounded view of the visible terminal screen for the active session.",
            json!({
                "type": "object",
                "properties": {
                    "maxLines": { "type": "integer", "minimum": 1 }
                }
            }),
        ),
        tool_definition(
            "wait_for_text",
            "Wait until visible screen text appears or the timeout expires.",
            json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string" },
                    "timeoutMs": { "type": "integer", "minimum": 0 },
                    "maxLines": { "type": "integer", "minimum": 1 }
                },
                "required": ["text"]
            }),
        ),
        tool_definition(
            "resize",
            "Resize the live PTY session and return the updated bounded screen.",
            json!({
                "type": "object",
                "properties": {
                    "width": { "type": "integer", "minimum": 1 },
                    "height": { "type": "integer", "minimum": 1 },
                    "settleMs": { "type": "integer", "minimum": 0 },
                    "maxLines": { "type": "integer", "minimum": 1 }
                },
                "required": ["width", "height"]
            }),
        ),
        tool_definition(
            "screenshot",
            "Write a text screenshot artifact of the visible terminal screen.",
            json!({
                "type": "object",
                "properties": {
                    "label": { "type": "string" }
                }
            }),
        ),
        tool_definition(
            "read_save_metadata",
            "Read bounded metadata about save files under ~/.polit/saves.",
            json!({
                "type": "object",
                "properties": {
                    "maxEntries": { "type": "integer", "minimum": 1 }
                }
            }),
        ),
        tool_definition(
            "read_recent_logs",
            "Read a bounded tail of a whitelisted log file under ~/.polit/logs.",
            json!({
                "type": "object",
                "properties": {
                    "logKind": { "type": "string" },
                    "maxLines": { "type": "integer", "minimum": 1 }
                },
                "required": ["logKind"]
            }),
        ),
        tool_definition(
            "read_file_excerpt",
            "Read a bounded excerpt from a whitelisted config, log, or save file under ~/.polit.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "maxLines": { "type": "integer", "minimum": 1 }
                },
                "required": ["path"]
            }),
        ),
        tool_definition(
            "terminate",
            "Terminate the active live polit session.",
            json!({
                "type": "object",
                "properties": {}
            }),
        ),
    ]
}

fn tool_definition(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    })
}
