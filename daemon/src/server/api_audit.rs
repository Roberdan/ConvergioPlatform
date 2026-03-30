// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Audit APIs: project report + action audit trail.
// GET /api/audit/project/:project_id — see api_audit_project.rs
// GET /api/audit/log?limit=100&agent=<name> — action audit trail

use super::api_audit_project;
use super::state::{query_rows, ApiError, ServerState};
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn router() -> Router<ServerState> {
    Router::new()
        .merge(api_audit_project::router())
        .route("/api/audit/log", get(audit_log_handler))
}

/// Insert one row into `audit_log`. Best-effort: errors are logged, never propagated.
/// PooledConnection derefs to rusqlite::Connection via Deref — pass `conn` directly.
pub fn log_audit(
    conn: &rusqlite::Connection,
    agent: Option<&str>,
    action: &str,
    resource: Option<&str>,
    detail: Option<&str>,
) {
    if let Err(e) = conn.execute(
        "INSERT INTO audit_log (agent, action, resource, detail) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![agent, action, resource, detail],
    ) {
        tracing::warn!("audit_log insert failed: {e}");
    }
}

/// GET /api/audit/log?limit=100&agent=<name>
async fn audit_log_handler(
    State(state): State<ServerState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(100)
        .min(1000);

    let conn = state.get_conn()?;

    if !api_audit_project::table_exists(&conn, "audit_log") {
        return Ok(Json(json!({"entries": [], "total": 0})));
    }

    let entries = if let Some(agent) = params.get("agent") {
        query_rows(
            &conn,
            "SELECT id, timestamp, agent, action, resource, detail, ip_addr \
             FROM audit_log WHERE agent = ?1 ORDER BY id DESC LIMIT ?2",
            rusqlite::params![agent, limit],
        )?
    } else {
        query_rows(
            &conn,
            "SELECT id, timestamp, agent, action, resource, detail, ip_addr \
             FROM audit_log ORDER BY id DESC LIMIT ?1",
            rusqlite::params![limit],
        )?
    };

    let total = entries.len();
    Ok(Json(json!({"entries": entries, "total": total})))
}

