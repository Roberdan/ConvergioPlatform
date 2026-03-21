use super::api_heartbeat_handlers::collect_system_info;
use super::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/heartbeat", post(handle_heartbeat))
        .route("/api/heartbeat/status", get(handle_heartbeat_status))
        .route("/api/watchdog/status", get(handle_watchdog_status))
        .route("/api/watchdog/diagnostics", get(handle_diagnostics))
}

/// POST /api/heartbeat — extended heartbeat with task/agent counts
/// Body: {peer_name, load_json?, capabilities?, task_count?, agent_count?}
async fn handle_heartbeat(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let peer_name_owned = body
        .get("peer_name")
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(|| {
            hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string())
        });
    let peer_name = peer_name_owned.as_str();

    let load_json = body.get("load_json").cloned().unwrap_or(Value::Null);
    let capabilities = body
        .get("capabilities")
        .and_then(Value::as_str)
        .unwrap_or("");

    // Collect system metrics
    let sys_info = collect_system_info();
    let load = if load_json.is_null() {
        sys_info.clone()
    } else {
        load_json
    };

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let conn = state.get_conn()?;
    let conn = &conn;

    conn.execute(
        "INSERT OR REPLACE INTO peer_heartbeats \
         (peer_name, last_seen, load_json, capabilities) \
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            peer_name,
            now_secs,
            serde_json::to_string(&load).unwrap_or_default(),
            capabilities,
        ],
    )
    .map_err(|e| ApiError::internal(format!("heartbeat failed: {e}")))?;

    // Also update host_heartbeats if the table exists
    let _ = conn.execute(
        "INSERT OR REPLACE INTO host_heartbeats \
         (hostname, last_seen, status, metadata) \
         VALUES (?1, datetime('now'), 'online', ?2)",
        rusqlite::params![peer_name, serde_json::to_string(&load).unwrap_or_default()],
    );

    Ok(Json(json!({
        "ok": true,
        "peer_name": peer_name,
        "timestamp": now_secs,
    })))
}

/// GET /api/heartbeat/status — all peer heartbeat statuses
async fn handle_heartbeat_status(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let peers = query_rows(
        conn,
        "SELECT peer_name, last_seen, load_json, capabilities \
         FROM peer_heartbeats ORDER BY last_seen DESC",
        [],
    )?;

    let statuses: Vec<Value> = peers
        .iter()
        .map(|p| {
            let seen = p.get("last_seen").and_then(Value::as_f64).unwrap_or(0.0);
            let age_secs = now_secs - seen;
            let status = if age_secs < 60.0 {
                "healthy"
            } else if age_secs < 300.0 {
                "stale"
            } else {
                "offline"
            };
            json!({
                "peer_name": p.get("peer_name"),
                "status": status,
                "age_secs": age_secs as i64,
                "last_seen": seen,
            })
        })
        .collect();

    Ok(Json(json!({
        "ok": true,
        "peers": statuses,
    })))
}

/// GET /api/watchdog/status — self-healing watchdog status
async fn handle_watchdog_status(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    let uptime = state.started_at.elapsed().as_secs();

    // Check for stale in-progress tasks (>24h)
    let stale_tasks = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM tasks \
         WHERE status = 'in_progress' \
         AND started_at < datetime('now', '-24 hours')",
        [],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    // Check for orphan agents (running but no heartbeat >1h)
    let orphan_agents = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM agent_activity \
         WHERE status = 'running' \
         AND started_at < datetime('now', '-1 hours')",
        [],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    let sys_info = collect_system_info();

    Ok(Json(json!({
        "ok": true,
        "uptime_secs": uptime,
        "stale_tasks": stale_tasks,
        "orphan_agents": orphan_agents,
        "system": sys_info,
        "healthy": stale_tasks == 0 && orphan_agents == 0,
    })))
}

/// GET /api/watchdog/diagnostics — detailed diagnostics
async fn handle_diagnostics(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    let stale_tasks = query_rows(
        conn,
        "SELECT id, task_id, title, status, started_at, plan_id \
         FROM tasks WHERE status = 'in_progress' \
         AND started_at < datetime('now', '-24 hours') \
         ORDER BY started_at LIMIT 20",
        [],
    )?;

    let orphan_agents = query_rows(
        conn,
        "SELECT agent_id, agent_type, status, started_at \
         FROM agent_activity WHERE status = 'running' \
         AND started_at < datetime('now', '-1 hours') \
         ORDER BY started_at LIMIT 20",
        [],
    )?;

    let pending_notifications = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM notification_queue WHERE status = 'pending'",
        [],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    let active_plans = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM plans WHERE status = 'doing'",
        [],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    Ok(Json(json!({
        "ok": true,
        "stale_tasks": stale_tasks,
        "orphan_agents": orphan_agents,
        "pending_notifications": pending_notifications,
        "active_plans": active_plans,
        "version": env!("CARGO_PKG_VERSION"),
    })))
}
