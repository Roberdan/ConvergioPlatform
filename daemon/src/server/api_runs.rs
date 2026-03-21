// api_runs: execution_runs CRUD endpoints
// Handlers for update, pause, resume → api_runs_handlers.rs
use super::api_runs_handlers::{pause_run, resume_run, update_run};
use super::state::{ApiError, ServerState};
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/runs", get(list_runs).post(create_run))
        .route("/api/runs/:id", get(get_run).put(update_run))
        .route("/api/runs/:id/pause", post(pause_run))
        .route("/api/runs/:id/resume", post(resume_run))
}

async fn list_runs(
    State(state): State<ServerState>,
    Query(qs): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(s) = qs.get("status").filter(|v| !v.is_empty()) {
        conditions.push("r.status=?".to_string());
        params.push(s.clone());
    }

    let limit: i64 = qs
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(50)
        .min(100);

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT r.id, r.goal, r.team, r.status, r.result, r.cost_usd, r.agents_used, \
         r.plan_id, p.name AS plan_name, r.started_at, r.completed_at, r.duration_minutes, \
         r.context_path, r.paused_at \
         FROM execution_runs r \
         LEFT JOIN plans p ON r.plan_id = p.id \
         {where_clause} \
         ORDER BY r.id DESC \
         LIMIT {limit}"
    );

    let rows = conn
        .prepare(&sql)
        .and_then(|mut stmt| {
            let idx: Vec<&dyn rusqlite::ToSql> =
                params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
            stmt.query_map(idx.as_slice(), |row| {
                Ok(json!({
                    "id":               row.get::<_,i64>(0)?,
                    "goal":             row.get::<_,String>(1)?,
                    "team":             row.get::<_,Option<String>>(2)?,
                    "status":           row.get::<_,Option<String>>(3)?,
                    "result":           row.get::<_,Option<String>>(4)?,
                    "cost_usd":         row.get::<_,Option<f64>>(5)?,
                    "agents_used":      row.get::<_,Option<i64>>(6)?,
                    "plan_id":          row.get::<_,Option<i64>>(7)?,
                    "plan_name":        row.get::<_,Option<String>>(8)?,
                    "started_at":       row.get::<_,Option<String>>(9)?,
                    "completed_at":     row.get::<_,Option<String>>(10)?,
                    "duration_minutes": row.get::<_,Option<f64>>(11)?,
                    "context_path":     row.get::<_,Option<String>>(12)?,
                    "paused_at":        row.get::<_,Option<String>>(13)?,
                }))
            })
            .and_then(|mapped| mapped.collect::<Result<Vec<_>, _>>())
        })
        .map_err(|e| ApiError::internal(format!("list runs failed: {e}")))?;

    Ok(Json(Value::Array(rows)))
}

pub async fn get_run(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;

    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.goal, r.team, r.status, r.result, r.cost_usd, r.agents_used, \
             r.plan_id, p.name AS plan_name, r.started_at, r.completed_at, r.duration_minutes, \
             r.context_path, r.paused_at \
             FROM execution_runs r \
             LEFT JOIN plans p ON r.plan_id = p.id \
             WHERE r.id=?1",
        )
        .map_err(|e| ApiError::internal(format!("prepare get_run: {e}")))?;

    let run: Option<Value> = stmt
        .query_map(rusqlite::params![id], |row| {
            Ok(json!({
                "id":               row.get::<_,i64>(0)?,
                "goal":             row.get::<_,String>(1)?,
                "team":             row.get::<_,Option<String>>(2)?,
                "status":           row.get::<_,Option<String>>(3)?,
                "result":           row.get::<_,Option<String>>(4)?,
                "cost_usd":         row.get::<_,Option<f64>>(5)?,
                "agents_used":      row.get::<_,Option<i64>>(6)?,
                "plan_id":          row.get::<_,Option<i64>>(7)?,
                "plan_name":        row.get::<_,Option<String>>(8)?,
                "started_at":       row.get::<_,Option<String>>(9)?,
                "completed_at":     row.get::<_,Option<String>>(10)?,
                "duration_minutes": row.get::<_,Option<f64>>(11)?,
                "context_path":     row.get::<_,Option<String>>(12)?,
                "paused_at":        row.get::<_,Option<String>>(13)?,
            }))
        })
        .and_then(|mut rows| rows.next().transpose())
        .map_err(|e| ApiError::internal(format!("query get_run: {e}")))?;

    let mut run = run.ok_or_else(|| ApiError::bad_request(format!("run {id} not found")))?;

    // Compute delegation cost for this run's plan_id within its time window
    if let Some(plan_id) = run.get("plan_id").and_then(Value::as_i64) {
        let started_at = run
            .get("started_at")
            .and_then(Value::as_str)
            .unwrap_or("1970-01-01");
        let completed_at = run
            .get("completed_at")
            .and_then(Value::as_str)
            .map(|s| s.to_string())
            .unwrap_or_else(|| "datetime('now')".to_string());

        let cost_sql = format!(
            "SELECT COALESCE(SUM(cost_estimate),0) FROM delegation_log \
             WHERE plan_id=?1 AND created_at BETWEEN ?2 AND {completed_at}"
        );
        let delegation_cost: f64 = conn
            .query_row(&cost_sql, rusqlite::params![plan_id, started_at], |row| {
                row.get(0)
            })
            .unwrap_or(0.0);

        if let Some(obj) = run.as_object_mut() {
            obj.insert(
                "delegation_cost".to_string(),
                Value::from(delegation_cost),
            );
        }
    } else {
        if let Some(obj) = run.as_object_mut() {
            obj.insert("delegation_cost".to_string(), Value::from(0.0));
        }
    }

    Ok(Json(run))
}

#[derive(Deserialize)]
struct CreateRunBody {
    goal: String,
    #[serde(default)]
    plan_id: Option<i64>,
    #[serde(default)]
    context_path: Option<String>,
}

async fn create_run(
    State(state): State<ServerState>,
    Json(body): Json<CreateRunBody>,
) -> Result<Json<Value>, ApiError> {
    let goal = body.goal.trim().to_string();
    if goal.is_empty() {
        return Err(ApiError::bad_request("goal is required"));
    }
    let conn = state.get_conn()?;
    conn.execute(
        "INSERT INTO execution_runs (goal, plan_id, context_path) VALUES (?1, ?2, ?3)",
        rusqlite::params![goal, body.plan_id, body.context_path],
    )
    .map_err(|e| ApiError::internal(format!("create run failed: {e}")))?;
    let id = conn.last_insert_rowid();
    Ok(Json(json!({"id": id})))
}
