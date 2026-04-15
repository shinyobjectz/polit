pub mod session;

use std::error::Error;
use std::io::{BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use self::session::SessionManager;

pub fn run_stdio_server<R: BufRead, W: Write>(
    reader: R,
    mut writer: W,
) -> Result<(), Box<dyn Error>> {
    let mut sessions = SessionManager::default();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<RpcRequest>(&line) {
            Ok(request) => sessions.handle(request),
            Err(error) => RpcResponse::invalid_request(Value::Null, error.to_string()),
        };

        serde_json::to_writer(&mut writer, &response)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct RpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
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
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn invalid_request(id: Value, message: String) -> Self {
        Self::error(id, -32600, message)
    }

    pub fn method_not_found(id: Value, method: &str) -> Self {
        Self::error(id, -32601, format!("unknown method '{method}'"))
    }

    pub fn internal_error(id: Value, message: String) -> Self {
        Self::error(id, -32603, message)
    }

    fn error(id: Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(RpcError { code, message }),
        }
    }
}
