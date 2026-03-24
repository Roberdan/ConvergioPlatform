// Actions — reusable functions for peer discovery, delegation, and event emission.

use crate::ipc::IpcEngine;
use std::path::Path;
use std::sync::Arc;

type AliResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

const DAEMON_BASE: &str = "http://localhost:8420";

/// Find an available online peer from mesh status.
/// Optionally exclude a specific peer (for retry after failure).
/// Retries up to 3 times with 2s backoff if HTTP not ready yet.
pub async fn find_available_peer(db_path: &Path, exclude: Option<&str>) -> Option<String> {
    let url = format!("{DAEMON_BASE}/api/mesh/status");
    let mut resp = None;
    for attempt in 0..3 {
        match reqwest::get(&url).await {
            Ok(r) => { resp = Some(r); break; }
            Err(e) => {
                if attempt < 2 {
                    tracing::debug!("ali: mesh status attempt {}, retrying in 2s: {e}", attempt + 1);
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                } else {
                    tracing::warn!("ali: mesh status failed after 3 attempts: {e}");
                    return None;
                }
            }
        }
    }
    let resp = resp?;

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("ali: mesh status parse failed: {e}");
            return None;
        }
    };

    let peers = body.get("peers")?.as_array()?;
    for peer in peers {
        let name = peer.get("peer_name")?.as_str()?;
        let online = peer.get("is_online").and_then(|v| v.as_bool()).unwrap_or(false);

        if online && exclude.map_or(true, |ex| ex != name) {
            return Some(name.to_string());
        }
    }

    // Fallback: check db_path parent for local peer config
    let _ = db_path; // used for future local-peer discovery
    None
}

/// Delegate a plan to the best available peer.
pub async fn delegate_plan(engine: &Arc<IpcEngine>, db_path: &Path, plan_id: i64) -> AliResult {
    let peer = find_available_peer(db_path, None).await;

    let Some(peer_name) = peer else {
        tracing::warn!("ali: no peers available for plan {plan_id}");
        emit(
            engine,
            "need_human",
            &serde_json::json!({
                "plan_id": plan_id,
                "reason": "no online peers available for delegation",
            }),
        )?;
        return Ok(());
    };

    delegate_to_peer(engine, plan_id, &peer_name).await
}

/// Delegate a plan to a specific peer via the mesh API.
pub async fn delegate_to_peer(engine: &Arc<IpcEngine>, plan_id: i64, peer: &str) -> AliResult {
    let url = format!("{DAEMON_BASE}/api/mesh/delegate");
    let payload = serde_json::json!({"plan_id": plan_id, "peer": peer});

    tracing::info!("ali: delegating plan {plan_id} to peer {peer}");

    let client = reqwest::Client::new();
    let resp = client.post(&url).json(&payload).send().await;

    match resp {
        Ok(r) if r.status().is_success() => {
            emit(
                engine,
                "plan_delegated",
                &serde_json::json!({"plan_id": plan_id, "peer": peer}),
            )?;
            Ok(())
        }
        Ok(r) => {
            let status = r.status();
            let body = r.text().await.unwrap_or_default();
            tracing::error!("ali: delegation failed: {status} — {body}");
            emit(
                engine,
                "delegation_failed",
                &serde_json::json!({
                    "plan_id": plan_id,
                    "peer": peer,
                    "reason": format!("HTTP {status}: {body}"),
                }),
            )?;
            Ok(())
        }
        Err(e) => {
            tracing::error!("ali: delegation request failed: {e}");
            emit(
                engine,
                "delegation_failed",
                &serde_json::json!({
                    "plan_id": plan_id,
                    "peer": peer,
                    "reason": e.to_string(),
                }),
            )?;
            Ok(())
        }
    }
}

/// Check for sibling plans that are now unblocked after a plan completes.
pub fn check_unblocked_plans(
    engine: &IpcEngine,
    conn: &rusqlite::Connection,
    master_id: i64,
) -> AliResult {
    // Find child plans that might now have their dependencies met
    let mut stmt = conn.prepare(
        "SELECT id FROM plans WHERE parent_plan_id = ?1 AND status = 'todo'",
    )?;

    let plan_ids: Vec<i64> = stmt
        .query_map(rusqlite::params![master_id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    for pid in plan_ids {
        if crate::db::plan_hierarchy::dependencies_met(conn, pid)? {
            tracing::info!("ali: plan {pid} now unblocked under master {master_id}");
            emit(
                engine,
                "plan_ready",
                &serde_json::json!({"plan_id": pid}),
            )?;
        }
    }

    Ok(())
}

/// Emit a structured event to the #orchestration channel.
pub fn emit(
    engine: &IpcEngine,
    event_type: &str,
    payload: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut content = payload.clone();
    if let Some(obj) = content.as_object_mut() {
        obj.insert("type".to_string(), serde_json::json!(event_type));
    }
    engine.broadcast(
        super::ALI_AGENT,
        &content.to_string(),
        "event",
        Some(super::CHANNEL),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::IpcEngine;
    use std::sync::Arc;
    use tempfile::NamedTempFile;

    fn test_engine() -> (NamedTempFile, Arc<IpcEngine>) {
        let tmp = NamedTempFile::new().unwrap();
        let engine = Arc::new(IpcEngine::new(tmp.path().to_path_buf()));
        crate::ipc::ensure_ipc_schema(&engine.open_conn().unwrap()).unwrap();
        let _ = engine.channel_create(
            super::super::CHANNEL,
            Some("test"),
            super::super::ALI_AGENT,
        );
        (tmp, engine)
    }

    #[test]
    fn emit_broadcasts_to_channel() {
        let (_tmp, engine) = test_engine();
        let result = emit(
            &engine,
            "plan_delegated",
            &serde_json::json!({"plan_id": 42, "peer": "macProM1"}),
        );
        assert!(result.is_ok());

        let history = engine
            .history(None, Some(super::super::CHANNEL), 10, None)
            .unwrap();
        if let crate::ipc::IpcResponse::MessageList { messages } = history {
            assert!(!messages.is_empty(), "expected at least one message");
            let msg = &messages[0];
            assert!(msg.content.contains("plan_delegated"));
            assert!(msg.content.contains("42"));
        } else {
            panic!("expected MessageList");
        }
    }

    #[test]
    fn check_unblocked_with_no_children_succeeds() {
        let (tmp, engine) = test_engine();
        let conn = rusqlite::Connection::open(tmp.path()).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS plans (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'todo',
                parent_plan_id INTEGER,
                depends_on TEXT,
                execution_mode TEXT,
                tasks_done INTEGER DEFAULT 0,
                tasks_total INTEGER DEFAULT 0,
                project_id TEXT DEFAULT 'test'
            );",
        )
        .unwrap();

        let result = check_unblocked_plans(&engine, &conn, 999);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn find_peer_returns_none_when_daemon_not_running() {
        let tmp = NamedTempFile::new().unwrap();
        let peer = find_available_peer(tmp.path(), None).await;
        assert!(peer.is_none(), "should be None when daemon is not running");
    }
}
