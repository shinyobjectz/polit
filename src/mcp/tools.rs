use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{json, Value};

use super::inspect;
use super::pty_session::{PtySession, PtySessionConfig};
use super::session::{ActiveSession, SessionManager};
use super::RpcResponse;

pub(crate) fn launch(manager: &mut SessionManager, id: Value, params: Value) -> RpcResponse {
    match launch_impl(manager, id.clone(), params) {
        Ok(response) => response,
        Err(error) => RpcResponse::internal_error(id, error.to_string()),
    }
}

pub(crate) fn send_keys(manager: &mut SessionManager, id: Value, params: Value) -> RpcResponse {
    let request: SendKeysRequest = match serde_json::from_value(params) {
        Ok(request) => request,
        Err(error) => return RpcResponse::invalid_request(id, error.to_string()),
    };

    if request.keys.is_empty() && request.text.as_deref().unwrap_or_default().is_empty() {
        return RpcResponse::invalid_request(id, "send_keys requires keys or text".to_string());
    }

    let response_id = id.clone();
    manager.with_session(id, move |session| {
        let settle_timeout = duration_from_millis(request.settle_ms.unwrap_or(750));
        if let Some(text) = request.text.as_deref() {
            session.runtime.type_text(text, settle_timeout)?;
        }
        for key in &request.keys {
            session.runtime.send_key(key, settle_timeout)?;
        }

        Ok(RpcResponse::success(
            response_id,
            screen_payload("ok", session, request.max_lines.unwrap_or(12)),
        ))
    })
}

pub(crate) fn read_screen(manager: &mut SessionManager, id: Value, params: Value) -> RpcResponse {
    let request: ReadScreenRequest = match serde_json::from_value(params) {
        Ok(request) => request,
        Err(error) => return RpcResponse::invalid_request(id, error.to_string()),
    };

    let response_id = id.clone();
    manager.with_session(id, move |session| {
        session.runtime.settle_for(Duration::from_millis(100));
        Ok(RpcResponse::success(
            response_id,
            screen_payload("ok", session, request.max_lines.unwrap_or(12)),
        ))
    })
}

pub(crate) fn wait_for_text(
    manager: &mut SessionManager,
    id: Value,
    params: Value,
) -> RpcResponse {
    let request: WaitForTextRequest = match serde_json::from_value(params) {
        Ok(request) => request,
        Err(error) => return RpcResponse::invalid_request(id, error.to_string()),
    };

    if request.text.trim().is_empty() {
        return RpcResponse::invalid_request(id, "wait_for_text requires text".to_string());
    }

    let response_id = id.clone();
    manager.with_session(id, move |session| {
        let found = match session
            .runtime
            .wait_for_text(&request.text, duration_from_millis(request.timeout_ms.unwrap_or(2000)))
        {
            Ok(()) => true,
            Err(error) if error.to_string().contains("expected text") => false,
            Err(error) => return Err(error),
        };

        let mut payload = screen_payload("ok", session, request.max_lines.unwrap_or(12));
        payload["found"] = json!(found);

        Ok(RpcResponse::success(response_id, payload))
    })
}

pub(crate) fn terminate(manager: &mut SessionManager, id: Value) -> RpcResponse {
    let Some(mut session) = manager.active_session.take() else {
        return RpcResponse::invalid_request(id, "no active session".to_string());
    };

    match session.runtime.terminate() {
        Ok(()) => RpcResponse::success(
            id,
            json!({
                "status": "terminated",
                "sessionActive": false,
            }),
        ),
        Err(error) => RpcResponse::internal_error(id, error.to_string()),
    }
}

pub(crate) fn resize(manager: &mut SessionManager, id: Value, params: Value) -> RpcResponse {
    let request: ResizeRequest = match serde_json::from_value(params) {
        Ok(request) => request,
        Err(error) => return RpcResponse::invalid_request(id, error.to_string()),
    };

    if request.width == 0 || request.height == 0 {
        return RpcResponse::invalid_request(id, "resize requires positive dimensions".to_string());
    }

    let response_id = id.clone();
    manager.with_session(id, move |session| {
        session.runtime.resize(request.width, request.height)?;
        session
            .runtime
            .settle_for(duration_from_millis(request.settle_ms.unwrap_or(250)));
        Ok(RpcResponse::success(
            response_id,
            screen_payload("ok", session, request.max_lines.unwrap_or(12)),
        ))
    })
}

pub(crate) fn screenshot(manager: &mut SessionManager, id: Value, params: Value) -> RpcResponse {
    let request: ScreenshotRequest = match serde_json::from_value(params) {
        Ok(request) => request,
        Err(error) => return RpcResponse::invalid_request(id, error.to_string()),
    };

    let response_id = id.clone();
    manager.with_session(id, move |session| {
        let artifact_dir = session.home_path.join(".polit").join("mcp-artifacts");
        fs::create_dir_all(&artifact_dir)?;

        let label = sanitize_label(request.label.as_deref().unwrap_or("screen"));
        let artifact_path =
            artifact_dir.join(format!("{label}-r{}.txt", session.runtime.screen_revision()));
        fs::write(&artifact_path, session.runtime.screen_lines().join("\n"))?;

        Ok(RpcResponse::success(
            response_id,
            json!({
                "status": "ok",
                "sessionActive": true,
                "artifactPath": artifact_path,
                "screenRevision": session.runtime.screen_revision(),
            }),
        ))
    })
}

pub(crate) fn read_save_metadata(
    manager: &mut SessionManager,
    id: Value,
    params: Value,
) -> RpcResponse {
    let request: SaveMetadataRequest = match serde_json::from_value(params) {
        Ok(request) => request,
        Err(error) => return RpcResponse::invalid_request(id, error.to_string()),
    };

    let response_id = id.clone();
    manager.with_session(id, move |session| {
        let mut payload = inspect::read_save_metadata(
            &session.home_path,
            request.max_entries.unwrap_or(5),
        )?;
        payload["status"] = json!("ok");
        payload["sessionActive"] = json!(true);
        Ok(RpcResponse::success(response_id, payload))
    })
}

pub(crate) fn read_recent_logs(
    manager: &mut SessionManager,
    id: Value,
    params: Value,
) -> RpcResponse {
    let request: RecentLogsRequest = match serde_json::from_value(params) {
        Ok(request) => request,
        Err(error) => return RpcResponse::invalid_request(id, error.to_string()),
    };

    let response_id = id.clone();
    manager.with_session(id, move |session| {
        let mut payload = inspect::read_recent_logs(
            &session.home_path,
            &request.log_kind,
            request.max_lines.unwrap_or(20),
        )?;
        payload["status"] = json!("ok");
        payload["sessionActive"] = json!(true);
        Ok(RpcResponse::success(response_id, payload))
    })
}

pub(crate) fn read_file_excerpt(
    manager: &mut SessionManager,
    id: Value,
    params: Value,
) -> RpcResponse {
    let request: FileExcerptRequest = match serde_json::from_value(params) {
        Ok(request) => request,
        Err(error) => return RpcResponse::invalid_request(id, error.to_string()),
    };

    let response_id = id.clone();
    manager.with_session(id, move |session| {
        let mut payload = inspect::read_file_excerpt(
            &session.home_path,
            &request.path,
            request.max_lines.unwrap_or(20),
        )?;
        payload["status"] = json!("ok");
        payload["sessionActive"] = json!(true);
        Ok(RpcResponse::success(response_id, payload))
    })
}

pub(crate) fn not_implemented(
    manager: &SessionManager,
    id: Value,
    method: &str,
) -> RpcResponse {
    RpcResponse::success(
        id,
        json!({
            "status": "not_implemented",
            "method": method,
            "sessionActive": manager.active_session.is_some(),
        }),
    )
}

fn launch_impl(
    manager: &mut SessionManager,
    id: Value,
    params: Value,
) -> Result<RpcResponse, Box<dyn std::error::Error>> {
    let request: LaunchRequest = serde_json::from_value(params)?;

    if let Some(mut existing) = manager.active_session.take() {
        existing.runtime.terminate()?;
    }

    let binary_path = request.binary_path.unwrap_or(find_polit_binary()?);
    let home_path = request
        .home
        .unwrap_or_else(|| std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| ".".into()));
    let terminal_width = request
        .terminal
        .as_ref()
        .and_then(|terminal| terminal.width)
        .unwrap_or(100);
    let terminal_height = request
        .terminal
        .as_ref()
        .and_then(|terminal| terminal.height)
        .unwrap_or(30);

    let runtime = PtySession::launch(
        &binary_path,
        PtySessionConfig::new(&home_path, terminal_width, terminal_height)
            .with_path_env(request.path_env.unwrap_or_else(current_path_env))
            .with_args(request.args.unwrap_or_default()),
    )?;

    let mut active = ActiveSession {
        runtime,
        binary_path,
        home_path,
    };
    let payload = screen_payload("ok", &mut active, 12);
    manager.active_session = Some(active);

    Ok(RpcResponse::success(id, payload))
}

fn screen_payload(status: &str, session: &mut ActiveSession, max_lines: usize) -> Value {
    let lines = bounded_lines(session.runtime.screen_lines(), max_lines);
    let (width, height) = session.runtime.terminal_size().unwrap_or((0, 0));

    json!({
        "status": status,
        "sessionActive": true,
        "screenRevision": session.runtime.screen_revision(),
        "summary": summarize_screen(&lines),
        "lines": lines,
        "terminal": {
            "width": width,
            "height": height,
        },
        "binaryPath": session.binary_path,
        "home": session.home_path,
    })
}

fn summarize_screen(lines: &[String]) -> String {
    let summary_lines: Vec<&str> = lines
        .iter()
        .map(String::as_str)
        .filter(|line| !line.trim().is_empty())
        .take(3)
        .collect();
    if summary_lines.is_empty() {
        "<empty screen>".to_string()
    } else {
        summary_lines.join(" | ")
    }
}

fn bounded_lines(lines: Vec<String>, max_lines: usize) -> Vec<String> {
    lines.into_iter()
        .filter(|line| !line.trim().is_empty())
        .take(max_lines.max(1))
        .collect()
}

fn duration_from_millis(timeout_ms: u64) -> Duration {
    Duration::from_millis(timeout_ms.max(1))
}

fn find_polit_binary() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("POLIT_MCP_POLIT_BIN") {
        return Ok(path.into());
    }

    let current = std::env::current_exe()?;
    let sibling = if cfg!(windows) {
        current.with_file_name("polit.exe")
    } else {
        current.with_file_name("polit")
    };

    if sibling.exists() {
        Ok(sibling)
    } else {
        Err("unable to locate polit binary; set POLIT_MCP_POLIT_BIN".into())
    }
}

fn current_path_env() -> String {
    std::env::var("PATH").unwrap_or_default()
}

fn sanitize_label(label: &str) -> String {
    let mut cleaned = String::new();
    for ch in label.chars() {
        if ch.is_ascii_alphanumeric() {
            cleaned.push(ch.to_ascii_lowercase());
        } else if (ch == '-' || ch == '_') && !cleaned.ends_with('-') {
            cleaned.push('-');
        }
    }

    let cleaned = cleaned.trim_matches('-');
    if cleaned.is_empty() {
        "screen".to_string()
    } else {
        cleaned.to_string()
    }
}

#[derive(Debug, Deserialize)]
struct LaunchRequest {
    #[serde(rename = "binaryPath")]
    binary_path: Option<PathBuf>,
    home: Option<PathBuf>,
    #[serde(rename = "pathEnv")]
    path_env: Option<String>,
    args: Option<Vec<String>>,
    terminal: Option<TerminalRequest>,
}

#[derive(Debug, Deserialize)]
struct TerminalRequest {
    width: Option<u16>,
    height: Option<u16>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct SendKeysRequest {
    keys: Vec<String>,
    text: Option<String>,
    #[serde(rename = "settleMs")]
    settle_ms: Option<u64>,
    #[serde(rename = "maxLines")]
    max_lines: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ReadScreenRequest {
    #[serde(rename = "maxLines")]
    max_lines: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct WaitForTextRequest {
    text: String,
    #[serde(rename = "timeoutMs")]
    timeout_ms: Option<u64>,
    #[serde(rename = "maxLines")]
    max_lines: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ResizeRequest {
    width: u16,
    height: u16,
    #[serde(rename = "settleMs")]
    settle_ms: Option<u64>,
    #[serde(rename = "maxLines")]
    max_lines: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ScreenshotRequest {
    label: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct SaveMetadataRequest {
    #[serde(rename = "maxEntries")]
    max_entries: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RecentLogsRequest {
    #[serde(rename = "logKind")]
    log_kind: String,
    #[serde(rename = "maxLines")]
    max_lines: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct FileExcerptRequest {
    path: String,
    #[serde(rename = "maxLines")]
    max_lines: Option<usize>,
}
