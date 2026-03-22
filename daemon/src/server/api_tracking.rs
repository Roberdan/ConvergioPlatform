// Copyright (c) 2026 Roberto D'Angelo
//! Tracking API — hooks write token_usage, agent_activity, session_state, compaction.
//! Replaces direct sqlite3 calls in 5 hook scripts with HTTP POST endpoints.
use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/tracking/tokens", post(handle_tokens))
        .route("/api/tracking/agent-activity", post(handle_agent_activity))
        .route("/api/tracking/session-state", post(handle_session_state))
        .route("/api/tracking/compaction", post(handle_compaction))
}

/// POST /api/tracking/tokens — insert a token_usage row.
/// Body: {agent, model, input_tokens, output_tokens, cost_usd,
///        project_id?, plan_id?, wave_id?, task_id?, execution_host?}
async fn handle_tokens(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let agent = body
        .get("agent")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing agent"))?;
    let model = body
        .get("model")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing model"))?;
    let input_tokens = body
        .get("input_tokens")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request("missing input_tokens"))?;
    let output_tokens = body
        .get("output_tokens")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request("missing output_tokens"))?;
    let cost_usd = body
        .get("cost_usd")
        .and_then(Value::as_f64)
        .ok_or_else(|| ApiError::bad_request("missing cost_usd"))?;

    let project_id = body.get("project_id").and_then(Value::as_str);
    let plan_id = body.get("plan_id").and_then(Value::as_i64);
    let wave_id = body.get("wave_id").and_then(Value::as_str);
    let task_id = body.get("task_id").and_then(Value::as_str);
    let execution_host = body.get("execution_host").and_then(Value::as_str);

    let conn = state.get_conn()?;
    conn.execute(
        "INSERT INTO token_usage \
         (project_id, plan_id, wave_id, task_id, agent, model, \
          input_tokens, output_tokens, cost_usd, execution_host) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            project_id,
            plan_id,
            wave_id,
            task_id,
            agent,
            model,
            input_tokens,
            output_tokens,
            cost_usd,
            execution_host
        ],
    )
    .map_err(|e| ApiError::internal(format!("token_usage insert failed: {e}")))?;

    let id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .unwrap_or(0);

    Ok(Json(json!({
        "ok": true,
        "id": id,
        "agent": agent,
        "model": model,
        "input_tokens": input_tokens,
        "output_tokens": output_tokens,
    })))
}

/// POST /api/tracking/agent-activity — insert or update an agent_activity row.
/// Upserts on agent_id (unique). Body: {agent_id, action, status?, plan_id?,
/// task_db_id?, model?, tokens_in?, tokens_out?, tokens_total?, cost_usd?,
/// description?, started_at?, completed_at?, duration_s?, host?, metadata?}
async fn handle_agent_activity(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let agent_id = body
        .get("agent_id")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing agent_id"))?;
    let action = body
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing action"))?;

    let status = body
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("completed");
    let plan_id = body.get("plan_id").and_then(Value::as_i64);
    let task_db_id = body.get("task_db_id").and_then(Value::as_i64);
    let model = body.get("model").and_then(Value::as_str);
    let description = body.get("description").and_then(Value::as_str);
    let tokens_in = body.get("tokens_in").and_then(Value::as_i64).unwrap_or(0);
    let tokens_out = body.get("tokens_out").and_then(Value::as_i64).unwrap_or(0);
    let tokens_total = body
        .get("tokens_total")
        .and_then(Value::as_i64)
        .unwrap_or(tokens_in + tokens_out);
    let cost_usd = body.get("cost_usd").and_then(Value::as_f64).unwrap_or(0.0);
    let started_at = body.get("started_at").and_then(Value::as_str);
    let completed_at = body.get("completed_at").and_then(Value::as_str);
    let duration_s = body.get("duration_s").and_then(Value::as_f64);
    let host = body.get("host").and_then(Value::as_str);
    let metadata = body.get("metadata").and_then(Value::as_str);

    let conn = state.get_conn()?;
    conn.execute(
        "INSERT INTO agent_activity \
         (agent_id, action, status, plan_id, task_db_id, model, description, \
          tokens_in, tokens_out, tokens_total, cost_usd, \
          started_at, completed_at, duration_s, host, metadata) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16) \
         ON CONFLICT(agent_id) DO UPDATE SET \
             action      = excluded.action, \
             status      = excluded.status, \
             plan_id     = COALESCE(excluded.plan_id, agent_activity.plan_id), \
             task_db_id  = COALESCE(excluded.task_db_id, agent_activity.task_db_id), \
             model       = COALESCE(excluded.model, agent_activity.model), \
             description = COALESCE(excluded.description, agent_activity.description), \
             tokens_in   = excluded.tokens_in, \
             tokens_out  = excluded.tokens_out, \
             tokens_total= excluded.tokens_total, \
             cost_usd    = excluded.cost_usd, \
             started_at  = COALESCE(excluded.started_at, agent_activity.started_at), \
             completed_at= COALESCE(excluded.completed_at, agent_activity.completed_at), \
             duration_s  = COALESCE(excluded.duration_s, agent_activity.duration_s), \
             host        = COALESCE(excluded.host, agent_activity.host), \
             metadata    = COALESCE(excluded.metadata, agent_activity.metadata)",
        rusqlite::params![
            agent_id,
            action,
            status,
            plan_id,
            task_db_id,
            model,
            description,
            tokens_in,
            tokens_out,
            tokens_total,
            cost_usd,
            started_at,
            completed_at,
            duration_s,
            host,
            metadata
        ],
    )
    .map_err(|e| ApiError::internal(format!("agent_activity upsert failed: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "agent_id": agent_id,
        "action": action,
        "status": status,
    })))
}

/// POST /api/tracking/session-state — insert or replace a session_state key.
/// Body: {key, value}
async fn handle_session_state(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let key = body
        .get("key")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing key"))?;
    let value = body
        .get("value")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing value"))?;

    let conn = state.get_conn()?;
    conn.execute(
        "INSERT OR REPLACE INTO session_state (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, value],
    )
    .map_err(|e| ApiError::internal(format!("session_state upsert failed: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "key": key,
    })))
}

/// POST /api/tracking/compaction — log a context compaction event.
/// Body: {session_id?, event_type?, context?}
/// Writes to compaction_log table (created on first use if absent).
async fn handle_compaction(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let session_id = body.get("session_id").and_then(Value::as_str);
    let event_type = body
        .get("event_type")
        .and_then(Value::as_str)
        .unwrap_or("compaction");
    let context = body.get("context").and_then(Value::as_str);

    let conn = state.get_conn()?;

    // Create table if absent — compaction_log is not in the base schema yet.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compaction_log ( \
             id INTEGER PRIMARY KEY NOT NULL, \
             session_id TEXT, \
             event_type TEXT, \
             context TEXT, \
             created_at TEXT DEFAULT (datetime('now')) \
         )",
    )
    .map_err(|e| ApiError::internal(format!("compaction_log init failed: {e}")))?;

    conn.execute(
        "INSERT INTO compaction_log (session_id, event_type, context) \
         VALUES (?1, ?2, ?3)",
        rusqlite::params![session_id, event_type, context],
    )
    .map_err(|e| ApiError::internal(format!("compaction_log insert failed: {e}")))?;

    let id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .unwrap_or(0);

    Ok(Json(json!({
        "ok": true,
        "id": id,
        "event_type": event_type,
    })))
}
