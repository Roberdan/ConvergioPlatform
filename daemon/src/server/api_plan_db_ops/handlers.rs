use crate::server::state::{ApiError, ServerState, query_one};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use axum::extract::State;

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
        // intentional: nonexistent paths preserve the raw value for backward compatibility.
        .ok()
        .and_then(|p| p.into_os_string().into_string().ok())
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/plan-db/wave/create", post(handle_wave_create))
        .route("/api/plan-db/wave/update", post(handle_wave_update))
        .route("/api/plan-db/kb-search", get(super::kb::handle_kb_search))
        .route("/api/plan-db/kb-write", post(super::kb::handle_kb_write))
}

/// POST /api/plan-db/wave/create — Body: {plan_id, wave_id, name}
    #[tracing::instrument(skip_all)]
pub async fn handle_wave_create(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let plan_id = body
        .get("plan_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request("missing plan_id"))?;
    let wid_raw = &body["wave_id"];
    let wave_id_str = wid_raw
        .as_str()
        .map(String::from)
        .or_else(|| wid_raw.as_i64().map(|n| format!("W{n}")))
        .ok_or_else(|| ApiError::bad_request("missing wave_id"))?;
    let name = body
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(&wave_id_str);
    let conn = state.get_conn()?;
    let wave_db_id: i64 = conn.query_row(
        "INSERT INTO waves (plan_id, wave_id, name, status) VALUES (?1, ?2, ?3, 'pending') RETURNING id",
        rusqlite::params![plan_id, wave_id_str, name], |row| row.get(0),
    ).map_err(|e| ApiError::internal(format!("wave insert failed: {e}")))?;

    Ok(Json(
        json!({"ok": true, "wave_db_id": wave_db_id, "plan_id": plan_id,
        "wave_id": wave_id_str, "name": name, "status": "pending"}),
    ))
}

/// POST /api/plan-db/wave/update — update wave status
/// Body: {wave_id, status, notes?}
    #[tracing::instrument(skip_all)]
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

    // Read current status BEFORE the update to detect whether the transition
    // to "done" is genuinely new (needed for waves_merged increment below).
    let prev_row = query_one(
        conn,
        "SELECT plan_id, status FROM waves WHERE id = ?1",
        rusqlite::params![wave_id],
    )?;
    if prev_row.is_none() {
        return Err(ApiError::bad_request(format!("wave {wave_id} not found")));
    }

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

    // Update plan stats when wave completes.
    // Only increment waves_merged if the previous status was NOT already "done"
    // — idempotent retries must not double-count the merge counter.
    if status == "done" {
        let plan_id = prev_row.as_ref().and_then(|v| v.get("plan_id").and_then(Value::as_i64));
        let prev_status = prev_row
            .as_ref()
            .and_then(|v| v.get("status").and_then(Value::as_str).map(String::from));

        if let Some(pid) = plan_id {
            let increment = if prev_status.as_deref() == Some("done") { 0i64 } else { 1i64 };
            conn.execute(
                "UPDATE plans SET tasks_done = \
                 (SELECT COUNT(*) FROM tasks WHERE plan_id = ?1 AND status = 'done'), \
                 waves_merged = COALESCE(waves_merged, 0) + ?2, \
                 updated_at = datetime('now') WHERE id = ?1",
                rusqlite::params![pid, increment],
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
