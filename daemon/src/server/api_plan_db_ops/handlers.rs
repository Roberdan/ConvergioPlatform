use crate::server::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

/// Resolve a project path via std::fs::canonicalize so symlinks and case
/// variations (macOS HFS+/APFS) are normalised before being persisted.
///
/// Returns None when the path is empty or does not exist — callers store
/// the raw value in that case so they remain backward compatible.
pub fn canonicalize_project_path(raw: &str) -> Option<String> {
    if raw.is_empty() {
        return None;
    }
    std::fs::canonicalize(raw)
        .ok()
        .and_then(|p| p.into_os_string().into_string().ok())
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/plan-db/wave/update", post(handle_wave_update))
        .route("/api/plan-db/kb-search", get(handle_kb_search))
        .route("/api/plan-db/kb-write", post(handle_kb_write))
}

/// POST /api/plan-db/wave/update — update wave status
/// Body: {wave_id, status, notes?}
pub async fn handle_wave_update(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let wave_id = body
        .get("wave_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request("missing wave_id"))?;
    let status = body
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing status"))?;

    let conn = state.get_conn()?;
    let conn = &conn;

    // Guard: if setting to done, all tasks must be done/cancelled/skipped
    if status == "done" {
        let pending = query_one(
            conn,
            "SELECT COUNT(*) AS c FROM tasks \
             WHERE wave_id_fk = ?1 AND status NOT IN ('done', 'cancelled', 'skipped')",
            rusqlite::params![wave_id],
        )?
        .and_then(|v| v.get("c").and_then(Value::as_i64))
        .unwrap_or(0);

        if pending > 0 {
            return Err(ApiError::bad_request(format!(
                "wave {wave_id} has {pending} incomplete tasks"
            )));
        }
    }

    let changed = conn
        .execute(
            "UPDATE waves SET status = ?1, \
             started_at = CASE WHEN ?1 = 'in_progress' AND started_at IS NULL \
               THEN datetime('now') ELSE started_at END, \
             completed_at = CASE WHEN ?1 = 'done' \
               THEN datetime('now') ELSE completed_at END \
             WHERE id = ?2",
            rusqlite::params![status, wave_id],
        )
        .map_err(|e| ApiError::internal(format!("wave update failed: {e}")))?;

    if changed == 0 {
        return Err(ApiError::bad_request(format!("wave {wave_id} not found")));
    }

    // Update plan stats when wave completes
    if status == "done" {
        let plan_id = query_one(
            conn,
            "SELECT plan_id FROM waves WHERE id = ?1",
            rusqlite::params![wave_id],
        )?
        .and_then(|v| v.get("plan_id").and_then(Value::as_i64));

        if let Some(pid) = plan_id {
            // Recount done tasks for the plan
            conn.execute(
                "UPDATE plans SET tasks_done = \
                 (SELECT COUNT(*) FROM tasks WHERE plan_id = ?1 AND status = 'done'), \
                 updated_at = datetime('now') WHERE id = ?1",
                rusqlite::params![pid],
            )
            .map_err(|e| ApiError::internal(format!("plan stats update failed: {e}")))?;
        }
    }

    Ok(Json(json!({
        "ok": true,
        "wave_id": wave_id,
        "status": status,
    })))
}

#[derive(Deserialize)]
pub struct KbSearchQuery {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

/// GET /api/plan-db/kb-search?q=term — search knowledge_base table
pub async fn handle_kb_search(
    State(state): State<ServerState>,
    Query(params): Query<KbSearchQuery>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    // Check if knowledge_base table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master \
             WHERE type='table' AND name='knowledge_base'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(false);

    if !table_exists {
        return Ok(Json(json!({
            "ok": true,
            "results": [],
            "query": params.q,
        })));
    }

    let pattern = format!("%{}%", params.q);
    let results = query_rows(
        conn,
        "SELECT id, domain, title, content, created_at, hit_count \
         FROM knowledge_base \
         WHERE title LIKE ?1 OR content LIKE ?1 OR domain LIKE ?1 \
         ORDER BY hit_count DESC, created_at DESC \
         LIMIT ?2",
        rusqlite::params![pattern, params.limit],
    )?;

    Ok(Json(json!({
        "ok": true,
        "results": results,
        "query": params.q,
        "count": results.len(),
    })))
}

/// POST /api/plan-db/kb-write — insert or update a knowledge_base entry
/// Body: {domain, title, content, tags?, confidence?}
pub async fn handle_kb_write(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let domain = body
        .get("domain")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing domain"))?;
    let title = body
        .get("title")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing title"))?;
    let content = body
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing content"))?;

    let conn = state.get_conn()?;

    // Try upsert; fall back to plain INSERT if unique constraint is absent
    conn.execute(
        "INSERT INTO knowledge_base (domain, title, content, created_at, hit_count) \
         VALUES (?1, ?2, ?3, datetime('now'), 0) \
         ON CONFLICT(domain, title) DO UPDATE SET \
           content = excluded.content, \
           hit_count = hit_count + 1",
        rusqlite::params![domain, title, content],
    )
    .or_else(|_| {
        conn.execute(
            "INSERT INTO knowledge_base (domain, title, content, created_at, hit_count) \
             VALUES (?1, ?2, ?3, datetime('now'), 0)",
            rusqlite::params![domain, title, content],
        )
    })
    .map_err(|e| ApiError::internal(format!("kb-write failed: {e}")))?;

    let id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .unwrap_or(0);

    Ok(Json(json!({
        "ok": true,
        "id": id,
        "domain": domain,
        "title": title,
    })))
}
