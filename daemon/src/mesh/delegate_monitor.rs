use crate::server::state::ServerState;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{info, warn};

const POLL_INTERVAL: Duration = Duration::from_secs(30);
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delegation {
    pub task_id: String,
    pub plan_id: i64,
    pub peer_name: String,
    pub peer_addr: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateCompletePayload {
    pub task_id: String,
    pub result: String,
    #[serde(default)]
    pub output: Option<String>,
}

/// Spawn background poller: every 30s queries remote daemons for active delegations.
pub fn spawn_monitor(state: ServerState) {
    tokio::spawn(async move {
        info!(
            "delegate_monitor: started (poll every {}s)",
            POLL_INTERVAL.as_secs()
        );
        let client = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .build()
            .unwrap_or_default();
        loop {
            tokio::time::sleep(POLL_INTERVAL).await;
            if let Err(e) = poll_delegated_tasks(&state, &client).await {
                warn!("delegate_monitor: poll cycle failed: {e}");
            }
        }
    });
}

async fn poll_delegated_tasks(state: &ServerState, client: &reqwest::Client) -> Result<(), String> {
    let delegations = load_active_delegations(state)?;
    if delegations.is_empty() {
        return Ok(());
    }
    info!(
        "delegate_monitor: polling {} active delegations",
        delegations.len()
    );
    for d in &delegations {
        match query_remote_agents(client, d).await {
            Ok(remote_status) if remote_status != d.status => {
                update_delegation_status(state, d, &remote_status)?;
                broadcast_status_change(state, d, &remote_status);
                if is_terminal(&remote_status) {
                    info!("delegate_monitor: task {} completed via polling", d.task_id);
                }
            }
            Err(e) => warn!(
                "delegate_monitor: poll {} for {}: {e}",
                d.peer_name, d.task_id
            ),
            _ => {}
        }
    }
    Ok(())
}

/// Process "delegate_complete" coordinator event (callback from remote node).
/// Wired into api_coordinator.rs handle_process_events.
pub fn handle_delegate_complete(state: &ServerState, payload: &Value) -> Result<String, String> {
    let parsed: DelegateCompletePayload = serde_json::from_value(payload.clone())
        .map_err(|e| format!("invalid delegate_complete payload: {e}"))?;
    let conn = state.get_conn().map_err(|e| format!("db: {e:?}"))?;
    let new_status = match parsed.result.as_str() {
        "failed" | "error" => "failed",
        _ => "done",
    };
    conn.execute(
        "UPDATE tasks SET status = ?1 WHERE id = ?2",
        rusqlite::params![new_status, parsed.task_id],
    )
    .map_err(|e| format!("task update: {e}"))?;
    conn.execute(
        "UPDATE agent_runs SET status = 'completed' WHERE task_id = ?1 AND status = 'running'",
        rusqlite::params![parsed.task_id],
    )
    .map_err(|e| format!("agent_runs update: {e}"))?;
    let _ = state.ws_tx.send(json!({
        "type": "delegate_complete",
        "task_id": parsed.task_id,
        "result": parsed.result,
        "output": parsed.output,
    }));
    info!(
        "delegate_monitor: callback for task {} ({})",
        parsed.task_id, new_status
    );
    Ok(format!(
        "delegate_complete: task {} -> {new_status}",
        parsed.task_id
    ))
}

fn load_active_delegations(state: &ServerState) -> Result<Vec<Delegation>, String> {
    let conn = state.get_conn().map_err(|e| format!("db: {e:?}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT ar.task_id, ar.plan_id, ar.peer_name, ar.status \
             FROM agent_runs ar \
             WHERE ar.peer_name IS NOT NULL AND ar.peer_name != '' AND ar.status = 'running'",
        )
        .map_err(|e| format!("prepare: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Delegation {
                task_id: row.get::<_, String>(0).unwrap_or_default(),
                plan_id: row.get::<_, i64>(1).unwrap_or(0),
                peer_name: row.get::<_, String>(2).unwrap_or_default(),
                peer_addr: String::new(),
                status: row.get::<_, String>(3).unwrap_or_default(),
            })
        })
        .map_err(|e| format!("query: {e}"))?;
    let mut delegations: Vec<Delegation> = rows.filter_map(|r| r.ok()).collect();
    for d in &mut delegations {
        d.peer_addr = resolve_peer_addr(&conn, &d.peer_name);
    }
    delegations.retain(|d| !d.peer_addr.is_empty());
    Ok(delegations)
}

fn resolve_peer_addr(conn: &rusqlite::Connection, peer_name: &str) -> String {
    conn.query_row(
        "SELECT peer_id FROM peer_heartbeats WHERE peer_id LIKE ?1 ORDER BY timestamp DESC LIMIT 1",
        rusqlite::params![format!("%{peer_name}%")],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .map(|id| format!("http://{id}:8420"))
    .unwrap_or_default()
}

async fn query_remote_agents(
    client: &reqwest::Client,
    delegation: &Delegation,
) -> Result<String, String> {
    let url = format!("{}/api/ipc/agents", delegation.peer_addr);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("GET {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}: {url}", resp.status()));
    }
    let body: Value = resp.json().await.map_err(|e| format!("JSON: {e}"))?;
    let agents = body
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for agent in &agents {
        let meta = agent.get("metadata").and_then(Value::as_str).unwrap_or("");
        if meta.contains(&delegation.task_id) {
            return Ok("running".to_string());
        }
    }
    // Agent no longer registered = completed (fallback detection)
    Ok("done".to_string())
}

fn is_terminal(status: &str) -> bool {
    matches!(status, "done" | "completed" | "failed" | "cancelled")
}

fn update_delegation_status(
    state: &ServerState,
    delegation: &Delegation,
    new_status: &str,
) -> Result<(), String> {
    let conn = state.get_conn().map_err(|e| format!("db: {e:?}"))?;
    let task_status = if is_terminal(new_status) {
        new_status
    } else {
        "in_progress"
    };
    conn.execute(
        "UPDATE tasks SET status = ?1 WHERE id = ?2",
        rusqlite::params![task_status, delegation.task_id],
    )
    .map_err(|e| format!("task update: {e}"))?;
    if is_terminal(new_status) {
        conn.execute(
            "UPDATE agent_runs SET status = ?1 WHERE task_id = ?2 AND status = 'running'",
            rusqlite::params![new_status, delegation.task_id],
        )
        .map_err(|e| format!("agent_runs update: {e}"))?;
    }
    Ok(())
}

fn broadcast_status_change(state: &ServerState, delegation: &Delegation, new_status: &str) {
    let _ = state.ws_tx.send(json!({
        "type": "delegate_status_change",
        "task_id": delegation.task_id,
        "plan_id": delegation.plan_id,
        "peer": delegation.peer_name,
        "status": new_status,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delegate_complete_payload_deserializes() {
        let val = json!({"task_id": "T6-03", "result": "success", "output": "all good"});
        let p: DelegateCompletePayload = serde_json::from_value(val).unwrap();
        assert_eq!(p.task_id, "T6-03");
        assert_eq!(p.result, "success");
        assert_eq!(p.output.as_deref(), Some("all good"));
    }

    #[test]
    fn delegate_complete_payload_minimal() {
        let val = json!({"task_id": "T1-01", "result": "done"});
        let p: DelegateCompletePayload = serde_json::from_value(val).unwrap();
        assert_eq!(p.task_id, "T1-01");
        assert!(p.output.is_none());
    }

    #[test]
    fn terminal_status_check() {
        assert!(is_terminal("done"));
        assert!(is_terminal("completed"));
        assert!(is_terminal("failed"));
        assert!(is_terminal("cancelled"));
        assert!(!is_terminal("running"));
        assert!(!is_terminal("pending"));
    }
}
