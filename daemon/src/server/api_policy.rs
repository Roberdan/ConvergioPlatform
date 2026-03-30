// Execution policy API — risk-based auto-progression configuration.
// GET  /api/policy/status        — list all policies (all projects)
// PUT  /api/policy/:project_id   — upsert policy entry for a project/risk_level

use axum::extract::{Path, State};
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::Deserialize;

use super::state::ServerState;
use crate::orchestrator::policy;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/policy/status", get(handle_status))
        .route("/api/policy/:project_id", put(handle_upsert))
}

async fn handle_status(State(state): State<ServerState>) -> Json<serde_json::Value> {
    let conn = match state.get_conn() {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    };
    if let Err(e) = policy::ensure_table(&conn) {
        return Json(serde_json::json!({"ok": false, "error": e.to_string()}));
    }
    let mut stmt = match conn.prepare(
        "SELECT id, project_id, risk_level, auto_progress, require_human, \
         require_double_validation FROM execution_policy \
         ORDER BY project_id, CASE risk_level WHEN 'LOW' THEN 0 WHEN 'MEDIUM' THEN 1 \
         WHEN 'HIGH' THEN 2 WHEN 'CRITICAL' THEN 3 ELSE 4 END",
    ) {
        Ok(s) => s,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    };
    let rows: Vec<serde_json::Value> = match stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "project_id": row.get::<_, String>(1)?,
            "risk_level": row.get::<_, String>(2)?,
            "auto_progress": row.get::<_, bool>(3)?,
            "require_human": row.get::<_, bool>(4)?,
            "require_double_validation": row.get::<_, bool>(5)?,
        }))
    }) {
        Ok(mapped) => mapped.filter_map(|r| r.ok()).collect(),
        Err(_) => vec![],
    };
    Json(serde_json::json!({"ok": true, "policies": rows}))
}

#[derive(Debug, Deserialize)]
struct UpsertRequest {
    risk_level: String,
    auto_progress: bool,
    require_human: bool,
    require_double_validation: bool,
}

async fn handle_upsert(
    State(state): State<ServerState>,
    Path(project_id): Path<String>,
    Json(body): Json<UpsertRequest>,
) -> Json<serde_json::Value> {
    // Validate risk level
    if policy::RiskLevel::from_str(&body.risk_level).is_none() {
        return Json(serde_json::json!({
            "ok": false,
            "error": format!("invalid risk_level '{}'; expected LOW|MEDIUM|HIGH|CRITICAL", body.risk_level)
        }));
    }
    let conn = match state.get_conn() {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    };
    if let Err(e) = policy::ensure_table(&conn) {
        return Json(serde_json::json!({"ok": false, "error": e.to_string()}));
    }
    let result = conn.execute(
        "INSERT INTO execution_policy
         (project_id, risk_level, auto_progress, require_human, require_double_validation)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(project_id, risk_level) DO UPDATE SET
             auto_progress             = excluded.auto_progress,
             require_human             = excluded.require_human,
             require_double_validation = excluded.require_double_validation",
        rusqlite::params![
            project_id,
            body.risk_level.to_uppercase(),
            body.auto_progress,
            body.require_human,
            body.require_double_validation,
        ],
    );
    match result {
        Ok(_) => Json(serde_json::json!({
            "ok": true,
            "project_id": project_id,
            "risk_level": body.risk_level.to_uppercase(),
        })),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}
