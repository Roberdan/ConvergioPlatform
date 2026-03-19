use super::state::{query_rows, ApiError, ServerState};
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

const PROPOSALS_TABLE: &str = "CREATE TABLE IF NOT EXISTS evolution_proposals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    hypothesis TEXT NOT NULL,
    target_metric TEXT NOT NULL,
    expected_delta REAL DEFAULT 0,
    blast_radius TEXT DEFAULT 'SingleRepo',
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK(status IN ('pending','approved','rejected','running','completed','rolled_back')),
    reviewer TEXT,
    reviewed_at TEXT,
    review_reason TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
)";

const EXPERIMENTS_TABLE: &str = "CREATE TABLE IF NOT EXISTS evolution_experiments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    proposal_id INTEGER NOT NULL REFERENCES evolution_proposals(id),
    mode TEXT NOT NULL DEFAULT 'canary',
    before_metrics TEXT,
    after_metrics TEXT,
    result TEXT DEFAULT 'pending'
        CHECK(result IN ('pending','success','failure','rolled_back')),
    started_at TEXT DEFAULT (datetime('now')),
    completed_at TEXT
)";

const AUDIT_TABLE: &str = "CREATE TABLE IF NOT EXISTS evolution_audit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    proposal_id INTEGER NOT NULL REFERENCES evolution_proposals(id),
    action TEXT NOT NULL,
    actor TEXT,
    reason TEXT,
    created_at TEXT DEFAULT (datetime('now'))
)";

fn ensure_tables(conn: &rusqlite::Connection) -> Result<(), ApiError> {
    conn.execute_batch(PROPOSALS_TABLE)
        .map_err(|e| ApiError::internal(format!("evolution_proposals create: {e}")))?;
    conn.execute_batch(EXPERIMENTS_TABLE)
        .map_err(|e| ApiError::internal(format!("evolution_experiments create: {e}")))?;
    conn.execute_batch(AUDIT_TABLE)
        .map_err(|e| ApiError::internal(format!("evolution_audit create: {e}")))?;
    Ok(())
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/evolution/proposals", get(list_proposals))
        .route(
            "/api/evolution/proposals/:id/approve",
            post(approve_proposal),
        )
        .route("/api/evolution/proposals/:id/reject", post(reject_proposal))
        .route("/api/evolution/experiments", get(list_experiments))
        .route("/api/evolution/roi", get(get_roi))
        .route("/api/evolution/audit/:id", get(get_audit_trail))
}

async fn list_proposals(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    ensure_tables(&conn)?;
    let rows = query_rows(
        &conn,
        "SELECT * FROM evolution_proposals ORDER BY created_at DESC",
        [],
    )?;
    Ok(Json(json!({ "proposals": rows })))
}

async fn approve_proposal(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    ensure_tables(&conn)?;
    let reason = body
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("approved via dashboard");
    let actor = body
        .get("actor")
        .and_then(Value::as_str)
        .unwrap_or("dashboard");
    let updated = conn
        .execute(
            "UPDATE evolution_proposals SET status='approved', reviewer=?1,
             reviewed_at=datetime('now'), review_reason=?2,
             updated_at=datetime('now') WHERE id=?3 AND status='pending'",
            rusqlite::params![actor, reason, id],
        )
        .map_err(|e| ApiError::internal(format!("approve failed: {e}")))?;
    if updated == 0 {
        return Err(ApiError::bad_request(
            "proposal not found or not in pending status",
        ));
    }
    conn.execute(
        "INSERT INTO evolution_audit (proposal_id, action, actor, reason)
         VALUES (?1, 'approve', ?2, ?3)",
        rusqlite::params![id, actor, reason],
    )
    .map_err(|e| ApiError::internal(format!("audit insert failed: {e}")))?;
    Ok(Json(json!({ "ok": true, "id": id, "status": "approved" })))
}

async fn reject_proposal(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    ensure_tables(&conn)?;
    let reason = body
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("rejected via dashboard");
    let actor = body
        .get("actor")
        .and_then(Value::as_str)
        .unwrap_or("dashboard");
    let updated = conn
        .execute(
            "UPDATE evolution_proposals SET status='rejected', reviewer=?1,
             reviewed_at=datetime('now'), review_reason=?2,
             updated_at=datetime('now') WHERE id=?3 AND status='pending'",
            rusqlite::params![actor, reason, id],
        )
        .map_err(|e| ApiError::internal(format!("reject failed: {e}")))?;
    if updated == 0 {
        return Err(ApiError::bad_request(
            "proposal not found or not in pending status",
        ));
    }
    conn.execute(
        "INSERT INTO evolution_audit (proposal_id, action, actor, reason)
         VALUES (?1, 'reject', ?2, ?3)",
        rusqlite::params![id, actor, reason],
    )
    .map_err(|e| ApiError::internal(format!("audit insert failed: {e}")))?;
    Ok(Json(json!({ "ok": true, "id": id, "status": "rejected" })))
}

async fn list_experiments(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    ensure_tables(&conn)?;
    let rows = query_rows(
        &conn,
        "SELECT e.*, p.hypothesis, p.target_metric
         FROM evolution_experiments e
         JOIN evolution_proposals p ON p.id = e.proposal_id
         ORDER BY e.started_at DESC",
        [],
    )?;
    Ok(Json(json!({ "experiments": rows })))
}

async fn get_roi(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    ensure_tables(&conn)?;
    let total = query_rows(
        &conn,
        "SELECT COUNT(*) AS total FROM evolution_experiments",
        [],
    )?;
    let successes = query_rows(
        &conn,
        "SELECT COUNT(*) AS successes FROM evolution_experiments
         WHERE result='success'",
        [],
    )?;
    let rollbacks = query_rows(
        &conn,
        "SELECT COUNT(*) AS rollbacks FROM evolution_experiments
         WHERE result='rolled_back'",
        [],
    )?;
    let proposals = query_rows(
        &conn,
        "SELECT status, COUNT(*) AS count FROM evolution_proposals
         GROUP BY status",
        [],
    )?;
    let extract_count = |rows: &[Value], key: &str| -> i64 {
        rows.first()
            .and_then(|v| v.get(key))
            .and_then(Value::as_i64)
            .unwrap_or(0)
    };
    let total_n = extract_count(&total, "total");
    let success_n = extract_count(&successes, "successes");
    let rollback_n = extract_count(&rollbacks, "rollbacks");
    let success_rate = if total_n > 0 {
        (success_n as f64 / total_n as f64) * 100.0
    } else {
        0.0
    };
    Ok(Json(json!({
        "experimentsRun": total_n,
        "successes": success_n,
        "rollbacks": rollback_n,
        "successRate": (success_rate * 100.0).round() / 100.0,
        "proposalsByStatus": proposals,
    })))
}

async fn get_audit_trail(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    ensure_tables(&conn)?;
    let rows = query_rows(
        &conn,
        "SELECT * FROM evolution_audit WHERE proposal_id=?1
         ORDER BY created_at DESC",
        [id],
    )?;
    Ok(Json(json!({ "audit": rows, "proposal_id": id })))
}
