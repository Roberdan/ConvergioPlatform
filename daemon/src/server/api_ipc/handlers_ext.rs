// handlers_ext: Plan 668 agent write handlers + T2-04 list/deregister
use super::super::state::{query_rows, ApiError, ServerState};
use super::super::ws_brain::{broadcast_brain_agent_update, broadcast_brain_session_update};
use super::ensure_ipc_schema;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct RegisterAgent {
    agent_id: String,
    host: String,
    #[serde(default = "default_agent_type")]
    agent_type: String,
    pid: Option<i64>,
    metadata: Option<String>,
    parent_agent: Option<String>,
}

fn default_agent_type() -> String {
    "claude".into()
}

#[derive(Deserialize)]
pub struct UnregisterAgent {
    agent_id: String,
    host: String,
}

#[derive(Deserialize)]
pub struct HeartbeatAgent {
    agent_id: String,
    host: String,
    current_task: Option<String>,
}

pub async fn api_ipc_agents_register(
    State(state): State<ServerState>,
    Json(body): Json<RegisterAgent>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;

    // Check if agent already exists before insert — acquire() only for new agents (F-09)
    let already_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM ipc_agents WHERE name = ?1 AND host = ?2",
            rusqlite::params![body.agent_id, body.host],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    conn.execute(
        "INSERT OR REPLACE INTO ipc_agents
         (name, host, agent_type, pid, metadata, parent_agent, registered_at, last_seen)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6,
                 strftime('%Y-%m-%dT%H:%M:%f','now'),
                 strftime('%Y-%m-%dT%H:%M:%f','now'))",
        rusqlite::params![
            body.agent_id,
            body.host,
            body.agent_type,
            body.pid,
            body.metadata,
            body.parent_agent
        ],
    )
    .map_err(|e| ApiError::internal(format!("agent register failed: {e}")))?;

    if let Err(e) = state.ws_tx.send(json!({
        "type": "agent_registered",
        "agent_id": body.agent_id,
        "host": body.host,
    })) {
        tracing::debug!("ws agent_registered broadcast (no subscribers): {e}");
    }

    // Only acquire power guard for genuinely new agents, not re-registrations
    if !already_exists {
        crate::power_guard::PowerGuard::acquire();
    }

    // Push live agent list + session state to brain viz
    broadcast_brain_agent_update(&state);
    broadcast_brain_session_update(&state);

    Ok(Json(json!({ "ok": true, "agent_id": body.agent_id })))
}

pub async fn api_ipc_agents_unregister(
    State(state): State<ServerState>,
    Json(body): Json<UnregisterAgent>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    conn.execute(
        "DELETE FROM ipc_agents WHERE name = ?1 AND host = ?2",
        rusqlite::params![body.agent_id, body.host],
    )
    .map_err(|e| ApiError::internal(format!("agent unregister failed: {e}")))?;

    // Only release power guard if a row was actually deleted (F-10)
    if conn.changes() > 0 {
        crate::power_guard::PowerGuard::release();
    }

    if let Err(e) = state.ws_tx.send(json!({
        "type": "agent_unregistered",
        "agent_id": body.agent_id,
        "host": body.host,
    })) {
        tracing::debug!("ws agent_unregistered broadcast (no subscribers): {e}");
    }

    // Push updated agent list + session state to brain viz
    broadcast_brain_agent_update(&state);
    broadcast_brain_session_update(&state);

    Ok(Json(json!({ "ok": true })))
}

/// GET /api/ipc/agents/list — all registered sessions with derived status.
pub async fn api_ipc_agents_list(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    // Derive status: active if last_seen within 10 minutes, else inactive.
    let rows = query_rows(
        &conn,
        "SELECT name,
                agent_type AS type,
                CASE WHEN last_seen >= strftime('%Y-%m-%dT%H:%M:%f','now','-10 minutes')
                     THEN 'active' ELSE 'inactive' END AS status,
                pid,
                registered_at,
                parent_agent
         FROM ipc_agents ORDER BY last_seen DESC",
        [],
    )?;
    Ok(Json(json!({ "ok": true, "agents": rows })))
}

#[derive(Deserialize)]
pub struct DeregisterAgent {
    name: String,
}

/// POST /api/ipc/agents/deregister — remove an agent by name (all hosts).
pub async fn api_ipc_agents_deregister(
    State(state): State<ServerState>,
    Json(body): Json<DeregisterAgent>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let deleted = conn
        .execute(
            "DELETE FROM ipc_agents WHERE name = ?1",
            rusqlite::params![body.name],
        )
        .map_err(|e| ApiError::internal(format!("agent deregister failed: {e}")))?;

    // Release power guard once per deleted row — batch DELETE may remove N agents (F-11)
    for _ in 0..deleted {
        crate::power_guard::PowerGuard::release();
    }

    if let Err(e) = state.ws_tx.send(json!({
        "type": "agent_deregistered",
        "name": body.name,
    })) {
        tracing::debug!("ws agent_deregistered broadcast (no subscribers): {e}");
    }

    broadcast_brain_agent_update(&state);
    broadcast_brain_session_update(&state);

    Ok(Json(json!({ "ok": true, "deleted": deleted })))
}

pub async fn api_ipc_agents_heartbeat(
    State(state): State<ServerState>,
    Json(body): Json<HeartbeatAgent>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    // Update last_seen timestamp; store current_task in metadata JSON
    let metadata = body
        .current_task
        .as_deref()
        .map(|t| serde_json::json!({"current_task": t}).to_string());
    conn.execute(
        "UPDATE ipc_agents SET last_seen = strftime('%Y-%m-%dT%H:%M:%f','now'),
         metadata = COALESCE(?3, metadata)
         WHERE name = ?1 AND host = ?2",
        rusqlite::params![body.agent_id, body.host, metadata],
    )
    .map_err(|e| ApiError::internal(format!("agent heartbeat failed: {e}")))?;

    Ok(Json(json!({ "ok": true })))
}

/// GET /api/ipc/agents/tree — hierarchical agent tree (parentage).
pub async fn api_ipc_agents_tree(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let rows = query_rows(
        &conn,
        "SELECT name, host, agent_type, pid, parent_agent,
                CASE WHEN last_seen >= strftime('%Y-%m-%dT%H:%M:%f','now','-10 minutes')
                     THEN 'active' ELSE 'inactive' END AS status,
                registered_at, last_seen
         FROM ipc_agents ORDER BY registered_at ASC",
        [],
    )?;

    // Build tree: group by parent_agent
    let mut roots: Vec<Value> = Vec::new();
    let mut children_map: std::collections::HashMap<String, Vec<Value>> =
        std::collections::HashMap::new();

    for row in &rows {
        let name = row["name"].as_str().unwrap_or("");
        let parent = row["parent_agent"].as_str();
        let node = json!({
            "name": name,
            "host": row["host"],
            "type": row["agent_type"],
            "pid": row["pid"],
            "status": row["status"],
            "registered_at": row["registered_at"],
            "last_seen": row["last_seen"],
            "children": [],
        });
        match parent {
            Some(p) if !p.is_empty() => {
                children_map.entry(p.to_string()).or_default().push(node);
            }
            _ => roots.push(node),
        }
    }

    fn attach_children(node: &mut Value, children_map: &std::collections::HashMap<String, Vec<Value>>) {
        let name = node["name"].as_str().unwrap_or("").to_string();
        if let Some(kids) = children_map.get(&name) {
            let mut kids = kids.clone();
            for kid in &mut kids {
                attach_children(kid, children_map);
            }
            node["children"] = json!(kids);
        }
    }

    for root in &mut roots {
        attach_children(root, &children_map);
    }

    // Orphaned children whose parent is not registered become roots
    let known_names: std::collections::HashSet<String> = rows
        .iter()
        .filter_map(|r| r["name"].as_str().map(String::from))
        .collect();
    for (parent, kids) in &children_map {
        if !known_names.contains(parent) {
            roots.extend(kids.clone());
        }
    }

    Ok(Json(json!({ "ok": true, "tree": roots, "total": rows.len() })))
}
