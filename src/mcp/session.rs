use std::path::PathBuf;

use serde_json::{json, Value};

use super::pty_session::PtySession;
use super::tools;
use super::{RpcRequest, RpcResponse};

#[derive(Default)]
pub struct SessionManager {
    pub(crate) active_session: Option<ActiveSession>,
}

pub(crate) struct ActiveSession {
    pub(crate) runtime: PtySession,
    pub(crate) binary_path: PathBuf,
    pub(crate) home_path: PathBuf,
}

impl SessionManager {
    pub fn handle(&mut self, request: RpcRequest) -> Option<RpcResponse> {
        if request.jsonrpc != "2.0" {
            return Some(RpcResponse::invalid_request(
                request.id,
                "only jsonrpc 2.0 requests are supported".to_string(),
            ));
        }

        match request.method.as_str() {
            "initialize" => Some(RpcResponse::success(
                request.id,
                json!({
                    "protocolVersion": "2025-06-18",
                    "capabilities": {
                        "tools": {
                            "listChanged": false
                        }
                    },
                    "serverInfo": {
                        "name": "polit",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }),
            )),
            "notifications/initialized" => None,
            "ping" => Some(RpcResponse::success(request.id, json!({}))),
            "tools/list" => Some(RpcResponse::success(
                request.id,
                json!({
                    "tools": super::tool_definitions()
                }),
            )),
            "tools/call" => Some(self.handle_tool_call(request.id, request.params)),
            "launch" => Some(tools::launch(self, request.id, request.params)),
            "send_keys" => Some(tools::send_keys(self, request.id, request.params)),
            "read_screen" => Some(tools::read_screen(self, request.id, request.params)),
            "wait_for_text" => Some(tools::wait_for_text(self, request.id, request.params)),
            "resize" => Some(tools::resize(self, request.id, request.params)),
            "screenshot" => Some(tools::screenshot(self, request.id, request.params)),
            "read_save_metadata" => Some(tools::read_save_metadata(self, request.id, request.params)),
            "read_recent_logs" => Some(tools::read_recent_logs(self, request.id, request.params)),
            "read_file_excerpt" => Some(tools::read_file_excerpt(self, request.id, request.params)),
            "terminate" => Some(tools::terminate(self, request.id)),
            other => Some(RpcResponse::method_not_found(request.id, other)),
        }
    }

    fn handle_tool_call(&mut self, id: Option<Value>, params: Value) -> RpcResponse {
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return RpcResponse::invalid_request(id, "tools/call requires name".to_string());
        };
        let arguments = params.get("arguments").cloned().unwrap_or_else(|| json!({}));

        let response = match name {
            "launch" => tools::launch(self, Some(json!(0)), arguments),
            "send_keys" => tools::send_keys(self, Some(json!(0)), arguments),
            "read_screen" => tools::read_screen(self, Some(json!(0)), arguments),
            "wait_for_text" => tools::wait_for_text(self, Some(json!(0)), arguments),
            "resize" => tools::resize(self, Some(json!(0)), arguments),
            "screenshot" => tools::screenshot(self, Some(json!(0)), arguments),
            "read_save_metadata" => tools::read_save_metadata(self, Some(json!(0)), arguments),
            "read_recent_logs" => tools::read_recent_logs(self, Some(json!(0)), arguments),
            "read_file_excerpt" => tools::read_file_excerpt(self, Some(json!(0)), arguments),
            "terminate" => tools::terminate(self, Some(json!(0))),
            other => {
                return RpcResponse::success(
                    id,
                    json!({
                        "content": [{
                            "type": "text",
                            "text": format!("unknown tool '{other}'")
                        }],
                        "isError": true
                    }),
                )
            }
        };

        match (response.result, response.error) {
            (Some(result), None) => RpcResponse::success(
                id,
                json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&result)
                            .unwrap_or_else(|_| result.to_string())
                    }],
                    "structuredContent": result,
                    "isError": false
                }),
            ),
            (_, Some(error)) => RpcResponse::success(
                id,
                json!({
                    "content": [{
                        "type": "text",
                        "text": error.message
                    }],
                    "isError": true
                }),
            ),
            _ => RpcResponse::internal_error(id, "tool call produced no response".to_string()),
        }
    }

    pub(crate) fn with_session<F>(&mut self, id: Option<Value>, f: F) -> RpcResponse
    where
        F: FnOnce(&mut ActiveSession) -> Result<RpcResponse, Box<dyn std::error::Error>>,
    {
        let Some(session) = self.active_session.as_mut() else {
            return RpcResponse::invalid_request(id, "no active session".to_string());
        };

        match f(session) {
            Ok(response) => response,
            Err(error) => RpcResponse::internal_error(id, error.to_string()),
        }
    }
}
