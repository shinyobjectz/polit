use std::path::PathBuf;

use serde_json::Value;

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
    pub fn handle(&mut self, request: RpcRequest) -> RpcResponse {
        if request.jsonrpc != "2.0" {
            return RpcResponse::invalid_request(
                request.id,
                "only jsonrpc 2.0 requests are supported".to_string(),
            );
        }

        match request.method.as_str() {
            "launch" => tools::launch(self, request.id, request.params),
            "send_keys" => tools::send_keys(self, request.id, request.params),
            "read_screen" => tools::read_screen(self, request.id, request.params),
            "wait_for_text" => tools::wait_for_text(self, request.id, request.params),
            "resize" => tools::resize(self, request.id, request.params),
            "screenshot" => tools::screenshot(self, request.id, request.params),
            "read_save_metadata" => tools::read_save_metadata(self, request.id, request.params),
            "read_recent_logs" => tools::read_recent_logs(self, request.id, request.params),
            "read_file_excerpt" => tools::read_file_excerpt(self, request.id, request.params),
            "terminate" => tools::terminate(self, request.id),
            other => RpcResponse::method_not_found(request.id, other),
        }
    }

    pub(crate) fn with_session<F>(&mut self, id: Value, f: F) -> RpcResponse
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
