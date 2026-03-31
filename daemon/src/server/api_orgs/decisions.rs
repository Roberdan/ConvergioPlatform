use crate::server::api_ipc::ensure_ipc_schema;
use crate::server::state::{query_rows, ApiError, ServerState};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct LogDecisionRequest {
    pub decision: String,
    pub rationale: String,
    pub made_by: String,
    pub refs: Vec<String>,
}

#[derive(Deserialize)]
pub struct DecisionListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn ensure_decision_refs_column(conn: &rusqlite::Connection) {
    let _ = conn.execute_batch("ALTER TABLE ipc_decisions ADD COLUMN refs TEXT NOT NULL DEFAULT '[]';");
}

pub async fn log_decision(
    State(state): State<ServerState>,
    Path(org_id): Path<String>,
    Json(body): Json<LogDecisionRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    ensure_decision_refs_column(&conn);
    let id = format!("dec-{}", Uuid::new_v4().simple());
    conn.execute(
        "INSERT INTO ipc_decisions(id, org_id, decision, rationale, decided_by, refs)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            id,
            org_id,
            body.decision,
            body.rationale,
            body.made_by,
            serde_json::to_string(&body.refs)
                .map_err(|e| ApiError::bad_request(format!("invalid refs: {e}")))?,
        ],
    )
    .map_err(|e| ApiError::internal(format!("log decision failed: {e}")))?;
    Ok((StatusCode::CREATED, Json(json!({ "ok": true, "id": id }))))
}

pub async fn list_decisions(
    State(state): State<ServerState>,
    Path(org_id): Path<String>,
    Query(query): Query<DecisionListQuery>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    ensure_decision_refs_column(&conn);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let decisions = query_rows(
        &conn,
        "SELECT id, org_id, decision, rationale, decided_by AS made_by, refs, created_at
         FROM ipc_decisions
         WHERE org_id = ?1
         ORDER BY created_at DESC, rowid DESC
         LIMIT ?2 OFFSET ?3",
        rusqlite::params![org_id, limit, offset],
    )?;
    Ok(Json(json!({ "ok": true, "decisions": decisions, "limit": limit, "offset": offset })))
}
