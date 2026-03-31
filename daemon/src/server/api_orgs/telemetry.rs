use crate::server::api_ipc::ensure_ipc_schema;
use crate::server::state::{query_rows, ApiError, ServerState};
use axum::extract::{Path, Query, State};
use axum::{http::StatusCode, Json};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct RecordTelemetryRequest {
    pub agent: String,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub cost: f64,
    pub period: Option<String>,
}

#[derive(Deserialize)]
pub struct TelemetryQuery {
    pub period: Option<String>,
}

pub async fn record_telemetry(
    State(state): State<ServerState>,
    Path(org_id): Path<String>,
    Json(body): Json<RecordTelemetryRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let period = body.period.unwrap_or_else(|| "day".to_string());
    let row_id = format!("{}:{}:{}", org_id, body.agent, period);
    let tags = json!({
        "agent": body.agent,
        "tokens_in": body.tokens_in,
        "tokens_out": body.tokens_out,
        "cost": body.cost,
        "period": period
    })
    .to_string();
    conn.execute(
        "INSERT INTO ipc_org_telemetry(id, org_id, metric, value, tags)
         VALUES (?1, ?2, 'usage', ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET value=excluded.value, tags=excluded.tags, created_at=strftime('%Y-%m-%dT%H:%M:%f','now')",
        rusqlite::params![row_id, org_id, (body.tokens_in + body.tokens_out) as f64, tags],
    )
    .map_err(|e| ApiError::internal(format!("record telemetry failed: {e}")))?;
    Ok((StatusCode::CREATED, Json(json!({"ok": true}))))
}

pub async fn aggregate_telemetry(
    State(state): State<ServerState>,
    Path(org_id): Path<String>,
    Query(q): Query<TelemetryQuery>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let period = q.period.unwrap_or_else(|| "day".to_string());
    let rows = query_rows(
        &conn,
        "SELECT SUM(CAST(json_extract(tags, '$.tokens_in') AS INTEGER)) AS tokens_in,
                SUM(CAST(json_extract(tags, '$.tokens_out') AS INTEGER)) AS tokens_out,
                SUM(CAST(json_extract(tags, '$.cost') AS REAL)) AS cost
         FROM ipc_org_telemetry WHERE org_id=?1 AND metric='usage'
           AND json_extract(tags, '$.period')=?2",
        rusqlite::params![org_id, period],
    )?;
    Ok(Json(json!({"ok": true, "period": period, "aggregate": rows.first().cloned().unwrap_or(json!({}))})))
}

pub async fn per_agent_telemetry(
    State(state): State<ServerState>,
    Path(org_id): Path<String>,
    Query(q): Query<TelemetryQuery>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let period = q.period.unwrap_or_else(|| "day".to_string());
    let rows = query_rows(
        &conn,
        "SELECT json_extract(tags, '$.agent') AS agent,
                SUM(CAST(json_extract(tags, '$.tokens_in') AS INTEGER)) AS tokens_in,
                SUM(CAST(json_extract(tags, '$.tokens_out') AS INTEGER)) AS tokens_out,
                SUM(CAST(json_extract(tags, '$.cost') AS REAL)) AS cost
         FROM ipc_org_telemetry
         WHERE org_id=?1 AND metric='usage' AND json_extract(tags, '$.period')=?2
         GROUP BY json_extract(tags, '$.agent') ORDER BY tokens_out DESC",
        rusqlite::params![org_id, period],
    )?;
    Ok(Json(json!({"ok": true, "period": period, "agents": rows})))
}
