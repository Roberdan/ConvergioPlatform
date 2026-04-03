use super::super::state::{ApiError, ServerState};
use crate::mesh::peer_resolver;
use crate::orchestrator::delegation_pipeline;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/mesh/exec", post(handle_exec))
        .route("/api/mesh/delegate", post(handle_delegate))
}

/// POST /api/mesh/exec — execute command on remote peer via mesh
/// Body: {peer, command, args?, timeout_secs?}
    #[tracing::instrument(skip_all)]
async fn handle_exec(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let peer = body
        .get("peer")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing peer"))?;
    let command = body
        .get("command")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing command"))?;
    let args = body
        .get("args")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let timeout_secs = body
        .get("timeout_secs")
        .and_then(Value::as_u64)
        .unwrap_or(30);

    // Resolve peer through centralized resolver (B6 fix: SSH resolves from peers.conf)
    let resolved = match peer_resolver::resolve(peer) {
        Ok(resolved) => Some(resolved),
        Err(error) => {
            tracing::warn!(peer, %error, "intentional: peer resolution failed, falling back to raw peer host");
            None
        }
    };
    let connect_host = resolved
        .as_ref()
        .map(|r| r.tailscale_ip.clone())
        .filter(|ip| !ip.is_empty())
        .unwrap_or_else(|| peer.to_string());

    // Try HTTP first (daemon-to-daemon)
    let url = format!("http://{}:8420/api/health", connect_host);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| ApiError::internal(format!("http client failed: {e}")))?;

    let peer_reachable = client.get(&url).send().await.is_ok();

    if peer_reachable {
        // Forward command via daemon API
        let exec_url = format!("http://{}:8420/api/mesh/exec", connect_host);
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            client.post(&exec_url).json(&body).send(),
        )
        .await;

        match result {
            Ok(Ok(resp)) => {
                let json = resp
                    .json::<Value>()
                    .await
                    .unwrap_or(json!({"error": "parse failed"}));
                return Ok(Json(json!({
                    "ok": true, "peer": peer, "method": "http",
                    "result": json,
                })));
            }
            Ok(Err(e)) => {
                return Err(ApiError::internal(format!("http exec failed: {e}")));
            }
            Err(_) => {
                return Err(ApiError::internal("exec timeout"));
            }
        }
    }

    // Fallback: SSH exec (B6 fix: resolve SSH destination from peers.conf)
    let ssh_dest = resolved
        .as_ref()
        .map(|r| peer_resolver::ssh_destination(r))
        .unwrap_or_else(|| peer.to_string());
    let mut cmd_args = vec![ssh_dest, command.to_string()];
    for arg in &args {
        if let Some(s) = arg.as_str() {
            cmd_args.push(s.to_string());
        }
    }

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        tokio::process::Command::new("ssh").args(&cmd_args).output(),
    )
    .await
    .map_err(|_| ApiError::internal("ssh exec timeout"))?
    .map_err(|e| ApiError::internal(format!("ssh exec failed: {e}")))?;

    // Log event
    let conn = state.get_conn()?;
    if let Err(e) = conn.execute(
        "INSERT INTO coordinator_events (event_type, payload, source_node) \
         VALUES ('remote_exec', ?1, ?2)",
        rusqlite::params![
            json!({"peer": peer, "command": command}).to_string(),
            hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_default(),
        ],
    ) {
        tracing::warn!("coordinator remote_exec event insert failed: {e}");
    }

    Ok(Json(json!({
        "ok": output.status.success(),
        "peer": peer,
        "method": "ssh",
        "exit_code": output.status.code(),
        "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
        "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
    })))
}

/// POST /api/mesh/delegate — delegate plan execution to a peer
/// Body: {plan_id, peer, model?}
    #[tracing::instrument(skip_all)]
async fn handle_delegate(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let plan_id = body
        .get("plan_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request("missing plan_id"))?;
    let peer = body
        .get("peer")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing peer"))?;

    // B9 fix: canonicalize peer name before storing in DB — return 400 if unknown
    let canonical = peer_resolver::resolve(peer)
        .map(|r| r.canonical_name)
        .map_err(|e| ApiError::bad_request(format!("peer not found: {e}")))?;

    let conn = state.get_conn()?;
    let conn = &conn;

    // Update plan execution_host
    let changed = conn
        .execute(
            "UPDATE plans SET execution_host = ?1, updated_at = datetime('now') \
             WHERE id = ?2",
            rusqlite::params![canonical, plan_id],
        )
        .map_err(|e| ApiError::internal(format!("delegate failed: {e}")))?;

    if changed == 0 {
        return Err(ApiError::bad_request(format!("plan {plan_id} not found")));
    }

    // Log delegation event
    if let Err(e) = conn.execute(
        "INSERT INTO coordinator_events (event_type, payload, source_node) \
         VALUES ('plan_delegated', ?1, ?2)",
        rusqlite::params![
            json!({"plan_id": plan_id, "peer": peer}).to_string(),
            hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_default(),
        ],
    ) {
        tracing::warn!("coordinator plan_delegated event insert failed: {e}");
    }

    // Spawn full automated delegation pipeline as a background task.
    // The pipeline handles: binary rsync, repo rsync, worktree, plan sync,
    // daemon version check, and tmux launch — all automatic with timeouts.
    let pipeline_peer = canonical.clone();
    let db_path = state.db_path.clone();
    tokio::spawn(async move {
        match delegation_pipeline::run_full_delegation(plan_id, &pipeline_peer, &db_path).await {
            Ok(del_id) => {
                tracing::info!("delegation pipeline complete: plan {plan_id} → {pipeline_peer} ({del_id})");
            }
            Err(e) => {
                tracing::warn!("delegation pipeline failed: plan {plan_id} → {pipeline_peer}: {e}");
            }
        }
    });

    // Return stream URL for SSE progress monitoring
    let stream_url = format!(
        "/api/plan/delegate?plan_id={}&target={}&cli=copilot",
        plan_id, peer
    );

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "delegated_to": peer,
        "stream_url": stream_url,
    })))
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod tests;
