// api_runs_handlers: update, pause, and resume handlers for execution_runs
use super::api_runs::get_run;
use super::state::{ApiError, ServerState};
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct UpdateRunBody {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub agents_used: Option<i64>,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
}

pub async fn update_run(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateRunBody>,
) -> Result<Json<Value>, ApiError> {
    // Capture status before body is moved into the sync block
    let new_status = body.status.clone();
    // Perform DB update in a sync block — conn and vals are !Send, so must not
    // cross any await boundary. This block returns only the rows_changed count.
    let rows_changed = {
        let conn = state.get_conn()?;
        let mut sets: Vec<String> = Vec::new();
        let mut vals: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(v) = body.status {
            sets.push(format!("status=?{}", vals.len() + 1));
            vals.push(Box::new(v));
        }
        if let Some(v) = body.cost_usd {
            sets.push(format!("cost_usd=?{}", vals.len() + 1));
            vals.push(Box::new(v));
        }
        if let Some(v) = body.agents_used {
            sets.push(format!("agents_used=?{}", vals.len() + 1));
            vals.push(Box::new(v));
        }
        if let Some(v) = body.completed_at {
            sets.push(format!("completed_at=?{}", vals.len() + 1));
            vals.push(Box::new(v));
        }
        if let Some(v) = body.result {
            sets.push(format!("result=?{}", vals.len() + 1));
            vals.push(Box::new(v));
        }

        if sets.is_empty() {
            return Err(ApiError::bad_request("no fields to update"));
        }

        let id_idx = vals.len() + 1;
        let sql = format!("UPDATE execution_runs SET {} WHERE id=?{id_idx}", sets.join(", "));
        vals.push(Box::new(id));

        let idx: Vec<&dyn rusqlite::ToSql> = vals.iter().map(|v| v.as_ref()).collect();
        conn.execute(&sql, idx.as_slice())
            .map_err(|e| ApiError::internal(format!("update run failed: {e}")))?
        // conn, vals, idx all dropped here — no !Send types cross the await
    };

    if rows_changed == 0 {
        return Err(ApiError::bad_request(format!("run {id} not found")));
    }

    // Broadcast status change so dashboard updates in real time
    if let Some(status) = new_status {
        let _ = state.ws_tx.send(json!({
            "type": "run_update",
            "run_id": id,
            "status": status,
        }));
    }

    // Re-fetch from state to include plan_name and delegation_cost
    get_run(State(state), Path(id)).await
}

pub async fn pause_run(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    // conn is !Send — keep it in a sync block that drops before the .await
    let rows_changed = {
        let conn = state.get_conn()?;
        conn.execute(
            "UPDATE execution_runs SET status='paused', paused_at=datetime('now') WHERE id=?1",
            rusqlite::params![id],
        )
        .map_err(|e| ApiError::internal(format!("pause run failed: {e}")))?
    };

    if rows_changed == 0 {
        return Err(ApiError::bad_request(format!("run {id} not found")));
    }

    // Broadcast pause so dashboard updates in real time
    let _ = state.ws_tx.send(json!({
        "type": "run_update",
        "run_id": id,
        "status": "paused",
    }));

    get_run(State(state), Path(id)).await
}

pub async fn resume_run(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    // conn is !Send — keep it in a sync block that drops before the .await
    let rows_changed = {
        let conn = state.get_conn()?;
        conn.execute(
            "UPDATE execution_runs SET status='running', paused_at=NULL WHERE id=?1",
            rusqlite::params![id],
        )
        .map_err(|e| ApiError::internal(format!("resume run failed: {e}")))?
    };

    if rows_changed == 0 {
        return Err(ApiError::bad_request(format!("run {id} not found")));
    }

    // Broadcast resume so dashboard updates in real time
    let _ = state.ws_tx.send(json!({
        "type": "run_update",
        "run_id": id,
        "status": "running",
    }));

    get_run(State(state), Path(id)).await
}
