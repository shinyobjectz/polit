use serde_json::json;

use super::{RpcRequest, RpcResponse};

#[derive(Debug, Default)]
pub struct SessionManager {
    active_session: Option<ActiveSession>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveSession {
    terminal_width: u16,
    terminal_height: u16,
}

impl SessionManager {
    pub fn handle(&mut self, request: RpcRequest) -> RpcResponse {
        if request.jsonrpc != "2.0" {
            return RpcResponse::invalid_request(
                request.id,
                "only jsonrpc 2.0 requests are supported".to_string(),
            );
        }

        match request.method.as_str() {
            "launch" => self.recognized_placeholder(request, false),
            "send_keys" => self.recognized_placeholder(request, true),
            "read_screen" => self.recognized_placeholder(request, true),
            "wait_for_text" => self.recognized_placeholder(request, true),
            "resize" => self.recognized_placeholder(request, true),
            "screenshot" => self.recognized_placeholder(request, true),
            "read_save_metadata" => self.recognized_placeholder(request, true),
            "read_recent_logs" => self.recognized_placeholder(request, true),
            "read_file_excerpt" => self.recognized_placeholder(request, true),
            "terminate" => self.recognized_placeholder(request, true),
            other => RpcResponse::method_not_found(request.id, other),
        }
    }

    fn recognized_placeholder(
        &mut self,
        request: RpcRequest,
        session_required: bool,
    ) -> RpcResponse {
        if request.method == "launch" {
            self.active_session = Some(ActiveSession::from_launch_params(&request.params));
        }

        RpcResponse::success(
            request.id,
            json!({
                "status": "not_implemented",
                "method": request.method,
                "sessionActive": self.active_session.is_some(),
                "sessionRequired": session_required,
            }),
        )
    }
}

impl ActiveSession {
    fn from_launch_params(params: &serde_json::Value) -> Self {
        let terminal = params.get("terminal");
        let terminal_width = terminal
            .and_then(|value| value.get("width"))
            .and_then(|value| value.as_u64())
            .and_then(|value| u16::try_from(value).ok())
            .unwrap_or(100);
        let terminal_height = terminal
            .and_then(|value| value.get("height"))
            .and_then(|value| value.as_u64())
            .and_then(|value| u16::try_from(value).ok())
            .unwrap_or(30);

        Self {
            terminal_width,
            terminal_height,
        }
    }
}
