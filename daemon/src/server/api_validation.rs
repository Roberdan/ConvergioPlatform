// Validation queue API endpoints.
// GET  /api/validation/queue         — list queue entries
// GET  /api/validation/verdict/:task_id — get verdict for a task
// POST /api/validation/enqueue       — enqueue a validation request

use super::state::{ApiError, ServerState};
use crate::orchestrator::validator_service as vs;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/validation/queue", get(list_queue))
        .route("/api/validation/verdict/:task_id", get(get_verdict))
        .route("/api/validation/enqueue", post(enqueue))
        .route("/api/validation/record", post(record))
}

async fn list_queue(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    vs::run_migrations(&conn).map_err(|e| ApiError::internal(e.to_string()))?;
    let entries = vs::list_queue(&conn).map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(json!({ "queue": entries })))
}

async fn get_verdict(
    State(state): State<ServerState>,
    Path(task_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    vs::run_migrations(&conn).map_err(|e| ApiError::internal(e.to_string()))?;
    match vs::get_verdict(&conn, task_id).map_err(|e| ApiError::internal(e.to_string()))? {
        Some(v) => Ok(Json(json!({ "verdict": v }))),
        None => Ok(Json(json!({ "verdict": null }))),
    }
}

#[derive(Deserialize)]
struct EnqueueRequest {
    task_id: Option<i64>,
    wave_id: Option<i64>,
    plan_id: Option<i64>,
}

#[derive(Deserialize)]
struct RecordRequest {
    task_id: i64,
    verdict: String,
    report: Option<String>,
    validator: Option<String>,
}

/// POST /api/validation/record — enqueue + immediately record a verdict (Thor shortcut).
async fn record(
    State(state): State<ServerState>,
    Json(body): Json<RecordRequest>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    vs::run_migrations(&conn).map_err(|e| ApiError::internal(e.to_string()))?;
    let queue_id = vs::enqueue_validation(&conn, Some(body.task_id), None, None)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    vs::record_verdict(
        &conn,
        queue_id,
        &body.verdict,
        body.report.as_deref(),
        body.validator.as_deref(),
    )
    .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(json!({ "ok": true, "queue_id": queue_id, "verdict": body.verdict })))
}

async fn enqueue(
    State(state): State<ServerState>,
    Json(body): Json<EnqueueRequest>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    // Ensure tables exist on first use (idempotent).
    vs::run_migrations(&conn).map_err(|e| ApiError::internal(e.to_string()))?;
    let queue_id =
        vs::enqueue_validation(&conn, body.task_id, body.wave_id, body.plan_id)
            .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(json!({ "queue_id": queue_id })))
}
