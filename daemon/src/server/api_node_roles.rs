// Node role management — assign, list, and query node roles across the mesh.
// Roles determine which capabilities a node must have (kernel, executor, coordinator).

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use super::state::ServerState;

#[derive(Debug, Deserialize)]
pub struct AssignRoleRequest {
    pub node: String,
    pub role: String,
    pub capabilities: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct NodeRole {
    pub node: String,
    pub role: String,
    pub capabilities: Vec<String>,
    pub updated_at: String,
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/node/roles", get(list_roles))
        .route("/api/node/assign-role", post(assign_role))
}

async fn list_roles(State(state): State<ServerState>) -> Json<serde_json::Value> {
    let conn = match state.get_conn() {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    };
    // Ensure table exists
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS node_roles (
            node TEXT PRIMARY KEY,
            role TEXT NOT NULL DEFAULT 'executor',
            capabilities TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    );
    let mut stmt = conn
        .prepare("SELECT node, role, capabilities, updated_at FROM node_roles ORDER BY node")
        .unwrap();
    let roles: Vec<serde_json::Value> = stmt
        .query_map([], |row| {
            let caps_str: String = row.get(2)?;
            Ok(serde_json::json!({
                "node": row.get::<_, String>(0)?,
                "role": row.get::<_, String>(1)?,
                "capabilities": caps_str.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>(),
                "updated_at": row.get::<_, String>(3)?,
            }))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    Json(serde_json::json!({"ok": true, "roles": roles}))
}

async fn assign_role(
    State(state): State<ServerState>,
    Json(body): Json<AssignRoleRequest>,
) -> Json<serde_json::Value> {
    let conn = match state.get_conn() {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    };
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS node_roles (
            node TEXT PRIMARY KEY,
            role TEXT NOT NULL DEFAULT 'executor',
            capabilities TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    );
    let caps = body.capabilities.unwrap_or_default().join(",");
    let result = conn.execute(
        "INSERT INTO node_roles (node, role, capabilities, updated_at)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(node) DO UPDATE SET role=?2, capabilities=?3, updated_at=datetime('now')",
        rusqlite::params![body.node, body.role, caps],
    );
    match result {
        Ok(_) => Json(serde_json::json!({
            "ok": true, "node": body.node, "role": body.role
        })),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}
