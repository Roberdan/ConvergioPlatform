// Agent activity handlers — extracted from api_plan_db.rs (Plan F, T5-03).

use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

/// POST /api/plan-db/agent/start — register agent activity
/// Body: {agent_id, agent_type, description?, task_db_id?, plan_id?, model?, host?}
#[tracing::instrument(skip_all)]
pub(super) async fn handle_agent_start(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let agent_id = body
        .get("agent_id")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing agent_id"))?;
    let agent_type = body
        .get("agent_type")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let description = body
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");
    let task_db_id = body.get("task_db_id").and_then(Value::as_i64);
    let plan_id = body.get("plan_id").and_then(Value::as_i64);
    let model = body
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let host = body.get("host").and_then(Value::as_str).unwrap_or("local");

    let conn = state.get_conn()?;
    conn.execute(
        "INSERT OR REPLACE INTO agent_activity \
         (agent_id, agent_type, description, task_db_id, plan_id, model, host, \
          status, started_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'running', datetime('now'))",
        rusqlite::params![
            agent_id,
            agent_type,
            description,
            task_db_id,
            plan_id,
            model,
            host
        ],
    )
    .map_err(|e| ApiError::internal(format!("agent start failed: {e}")))?;
    Ok(Json(json!({"ok": true, "agent_id": agent_id})))
}

/// POST /api/plan-db/agent/complete — mark agent done
/// Body: {agent_id, tokens_in?, tokens_out?, cost_usd?, status?}
#[tracing::instrument(skip_all)]
pub(super) async fn handle_agent_complete(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let agent_id = body
        .get("agent_id")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing agent_id"))?;
    let status = body
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("completed");
    let tokens_in = body.get("tokens_in").and_then(Value::as_i64).unwrap_or(0);
    let tokens_out = body.get("tokens_out").and_then(Value::as_i64).unwrap_or(0);
    let cost = body.get("cost_usd").and_then(Value::as_f64).unwrap_or(0.0);

    let conn = state.get_conn()?;
    let changed = conn
        .execute(
            "UPDATE agent_activity SET status = ?1, tokens_in = ?2, tokens_out = ?3, \
         tokens_total = ?2 + ?3, cost_usd = ?4, completed_at = datetime('now'), \
         duration_s = ROUND((julianday('now') - julianday(started_at)) * 86400, 1) \
         WHERE agent_id = ?5",
            rusqlite::params![status, tokens_in, tokens_out, cost, agent_id],
        )
        .map_err(|e| ApiError::internal(format!("agent complete failed: {e}")))?;
    Ok(Json(
        json!({"ok": true, "agent_id": agent_id, "rows_changed": changed}),
    ))
}
