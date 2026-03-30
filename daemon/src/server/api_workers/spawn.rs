use super::super::state::{ApiError, ServerState};
use crate::mesh::peer_resolver;
use crate::server::ws_pty;
use axum::extract::State;
use axum::extract::{Path, Query};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::process::Command;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/delegate/spawn", post(handle_spawn))
        .route("/api/delegate/status", get(handle_status))
        .route("/api/delegate/:session_id", delete(handle_delete))
}

#[derive(Deserialize)]
struct SpawnRequest {
    #[serde(default = "default_peer")]
    peer: String,
    tmux_session: String,
    tmux_window: String,
    #[serde(default)]
    cwd: String,
    command: String,
}

#[derive(Deserialize)]
struct StatusQuery {
    #[serde(default = "default_session")]
    session: String,
}

fn default_peer() -> String {
    "local".to_string()
}

fn default_session() -> String {
    "Convergio".to_string()
}

fn quote_shell(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn sanitize_window(window: &str) -> Option<String> {
    ws_pty::sanitize_tmux_session(window)
}

fn build_tmux_script(body: &SpawnRequest) -> String {
    let session = quote_shell(&body.tmux_session);
    let window = quote_shell(&body.tmux_window);
    let cwd = if body.cwd.is_empty() {
        None
    } else {
        Some(quote_shell(&body.cwd))
    };
    let command = quote_shell(&body.command);
    let session_create = match cwd.as_deref() {
        Some(path) => format!("tmux new-session -d -s {session} -n kernel -c {path}"),
        None => format!("tmux new-session -d -s {session} -n kernel"),
    };
    let window_create = match cwd.as_deref() {
        Some(path) => format!(
            "tmux new-window -d -t {session} -n {window} -c {path} /bin/sh -lc {command}"
        ),
        None => format!("tmux new-window -d -t {session} -n {window} /bin/sh -lc {command}"),
    };
    format!("tmux has-session -t {session} 2>/dev/null || {session_create}; {window_create}")
}

fn run_shell(
    state: &ServerState,
    peer: &str,
    script: &str,
) -> Result<std::process::Output, ApiError> {
    if ws_pty::is_local_peer(state, peer) {
        Command::new("/bin/sh")
            .args(["-lc", script])
            .output()
            .map_err(|e| ApiError::internal(format!("local shell failed: {e}")))
    } else {
        let resolved = peer_resolver::resolve(peer)
            .map_err(|e| ApiError::bad_request(format!("unknown peer {peer}: {e}")))?;
        let ssh_target = peer_resolver::ssh_destination(&resolved);
        let remote_cmd = format!("$SHELL -lc {}", quote_shell(script));
        Command::new("ssh")
            .args(["-o", "ConnectTimeout=10", &ssh_target, &remote_cmd])
            .output()
            .map_err(|e| ApiError::internal(format!("remote shell failed: {e}")))
    }
}

#[tracing::instrument(skip_all)]
async fn handle_spawn(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let body: SpawnRequest = serde_json::from_value(body)
        .map_err(|e| ApiError::bad_request(format!("invalid spawn body: {e}")))?;
    if ws_pty::sanitize_tmux_session(&body.tmux_session).is_none() {
        return Err(ApiError::bad_request("invalid tmux_session"));
    }
    if sanitize_window(&body.tmux_window).is_none() {
        return Err(ApiError::bad_request("invalid tmux_window"));
    }
    if body.command.trim().is_empty() {
        return Err(ApiError::bad_request("missing command"));
    }

    let script = build_tmux_script(&body);
    let output = run_shell(&state, &body.peer, &script)?;
    let ok = output.status.success();

    if !ok {
        return Err(ApiError::internal("tmux spawn command failed"));
    }

    let pid_script = format!(
        "tmux list-panes -t {}:{} -F '#{{pane_pid}}' 2>/dev/null | head -1",
        body.tmux_session, body.tmux_window
    );
    let pid_output = run_shell(&state, &body.peer, &pid_script)?;
    // intentional: pane PID is best-effort metadata for the API response.
    let pid = String::from_utf8_lossy(&pid_output.stdout)
        .trim()
        .parse::<i64>()
        .ok(); // intentional: parse fallback to None when pane PID is not numeric

    Ok(Json(json!({
        "ok": true,
        "peer": body.peer,
        "session_id": format!("{}:{}", body.tmux_session, body.tmux_window),
        "tmux_session": body.tmux_session,
        "tmux_window": body.tmux_window,
        "window_name": body.tmux_window,
        "pid": pid,
    })))
}

async fn handle_status(
    State(state): State<ServerState>,
    Query(query): Query<StatusQuery>,
) -> Result<Json<Value>, ApiError> {
    let script = format!(
        "tmux list-windows -t {} -F '#{{window_name}}|#{{pane_pid}}' 2>/dev/null",
        query.session
    );
    let output = run_shell(&state, "local", &script)?;
    let windows = String::from_utf8_lossy(&output.stdout);
    let sessions = windows
        .lines()
        .filter_map(|line| {
            let (window, pid_str) = line.split_once('|')?;
            let pid = pid_str.parse::<i64>().ok()?; // intentional: skip malformed tmux lines instead of failing the full status listing
            let task_id = window.strip_prefix("plan-").and_then(|v| v.parse::<i64>().ok()); // intentional: non-plan windows have no task_id
            // intentional: runtime is auxiliary metadata; unknown maps to 0 seconds.
            let runtime = Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "etimes="])
                .output()
                .ok() // intentional: ps command failure means process already exited
                .and_then(|o| String::from_utf8(o.stdout).ok()) // intentional: non-UTF-8 ps output treated as absent
                .and_then(|s| s.trim().parse::<i64>().ok()) // intentional: parse fallback to None for non-numeric etimes
                .unwrap_or(0);
            Some(json!({
                "session_id": format!("{}:{}", query.session, window),
                "task_id": task_id,
                "window_name": window,
                "pid": pid,
                "runtime_secs": runtime,
                "status": if runtime > 0 { "running" } else { "unknown" },
            }))
        })
        .collect::<Vec<_>>();
    Ok(Json(json!({ "ok": true, "sessions": sessions })))
}

async fn handle_delete(
    State(state): State<ServerState>,
    Path(session_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let (session, window) = session_id
        .split_once(':')
        .ok_or_else(|| ApiError::bad_request("session_id must be session:window"))?;
    let script = format!("tmux kill-window -t {}:{} 2>/dev/null", session, window);
    let output = run_shell(&state, "local", &script)?;
    if !output.status.success() {
        return Err(ApiError::internal("tmux kill-window failed"));
    }
    Ok(Json(json!({ "ok": true, "session_id": session_id })))
}

#[cfg(test)]
mod tests {
    use super::{build_tmux_script, SpawnRequest};

    fn req() -> SpawnRequest {
        SpawnRequest {
            peer: "local".to_string(),
            tmux_session: "Convergio".to_string(),
            tmux_window: "plan-100".to_string(),
            cwd: "/tmp/worktree".to_string(),
            command: "echo ok".to_string(),
        }
    }

    #[test]
    fn tmux_script_contains_session_window_and_command() {
        let script = build_tmux_script(&req());
        assert!(script.contains("Convergio"));
        assert!(script.contains("plan-100"));
        assert!(script.contains("echo ok"));
        assert!(script.contains("tmux new-window"));
    }
}
