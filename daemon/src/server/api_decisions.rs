// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// F-27: Decision audit trail API.
// POST /api/decisions — log a decision with reasoning.
// GET  /api/decisions?plan_id=X — query decisions for a plan.

use super::state::{query_rows, ApiError, ServerState};
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/decisions", post(handle_log))
        .route("/api/decisions", get(handle_query))
}

/// POST /api/decisions — log a decision to decision_log.
/// Body: {decision, reasoning, plan_id?, task_id?, first_principles?,
///        alternatives_considered?, agent?}
    #[tracing::instrument(skip_all)]
pub async fn handle_log(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let decision = body
        .get("decision")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing decision"))?;
    let reasoning = body
        .get("reasoning")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing reasoning"))?;
    let plan_id = body.get("plan_id").and_then(Value::as_i64);
    let task_id = body.get("task_id").and_then(Value::as_i64);
    let first_principles = body.get("first_principles").and_then(Value::as_str);
    let alternatives = body.get("alternatives_considered").and_then(Value::as_str);
    let agent = body.get("agent").and_then(Value::as_str);

    let conn = state.get_conn()?;
    let id: i64 = conn
        .query_row(
            "INSERT INTO decision_log \
             (plan_id, task_id, decision, reasoning, first_principles, \
              alternatives_considered, agent) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) RETURNING id",
            rusqlite::params![
                plan_id, task_id, decision, reasoning,
                first_principles, alternatives, agent
            ],
            |row| row.get(0),
        )
        .map_err(|e| ApiError::internal(format!("decision_log insert failed: {e}")))?;

    Ok(Json(json!({ "ok": true, "id": id })))
}

#[derive(Debug, Deserialize)]
pub struct DecisionQuery {
    pub plan_id: Option<i64>,
    pub task_id: Option<i64>,
    pub agent: Option<String>,
    pub limit: Option<i64>,
}

/// GET /api/decisions?plan_id=X — query decisions, optionally filtered.
    #[tracing::instrument(skip_all)]
pub async fn handle_query(
    State(state): State<ServerState>,
    Query(params): Query<DecisionQuery>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let limit = params.limit.unwrap_or(50).min(200);

    // Build dynamic WHERE clause — all conditions are optional.
    let mut clauses: Vec<String> = vec![];
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = vec![];

    if let Some(pid) = params.plan_id {
        clauses.push(format!("plan_id = ?{}", clauses.len() + 1));
        values.push(Box::new(pid));
    }
    if let Some(tid) = params.task_id {
        clauses.push(format!("task_id = ?{}", clauses.len() + 1));
        values.push(Box::new(tid));
    }
    if let Some(ref ag) = params.agent {
        clauses.push(format!("agent = ?{}", clauses.len() + 1));
        values.push(Box::new(ag.clone()));
    }

    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    let sql = format!(
        "SELECT id, plan_id, task_id, decision, reasoning, first_principles, \
         alternatives_considered, outcome, created_at, agent \
         FROM decision_log {where_clause} \
         ORDER BY created_at DESC LIMIT {limit}"
    );

    let rows = query_rows(
        &conn,
        &sql,
        rusqlite::params_from_iter(values.iter().map(|v| v.as_ref())),
    )
    .map_err(|e| ApiError::internal(format!("decision_log query failed: {e}")))?;

    Ok(Json(json!({ "ok": true, "decisions": rows })))
}

#[cfg(test)]
mod tests {
    use crate::db::PlanDb;
    use crate::server::state::query_rows;

    fn setup_db() -> PlanDb {
        let db = PlanDb::open_in_memory().expect("db");
        db.connection()
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS decision_log (
                     id INTEGER PRIMARY KEY,
                     plan_id INTEGER, task_id INTEGER,
                     decision TEXT NOT NULL,
                     reasoning TEXT NOT NULL,
                     first_principles TEXT,
                     alternatives_considered TEXT,
                     outcome TEXT,
                     created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                     agent TEXT
                 );",
            )
            .expect("schema");
        db
    }

    #[test]
    fn decision_log_insert_and_query() {
        let db = setup_db();
        let conn = db.connection();
        conn.execute(
            "INSERT INTO decision_log (plan_id, task_id, decision, reasoning, agent) \
             VALUES (724, 9285, 'Use Ollama fallback', 'Ollama unavailable', 'task-executor')",
            [],
        )
        .unwrap();
        let rows = query_rows(
            conn,
            "SELECT id, decision FROM decision_log WHERE plan_id = 724",
            [],
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("decision").and_then(|v| v.as_str()), Some("Use Ollama fallback"));
    }

    #[test]
    fn decision_log_first_principles_stored() {
        let db = setup_db();
        let conn = db.connection();
        conn.execute(
            "INSERT INTO decision_log (decision, reasoning, first_principles) \
             VALUES ('restart', 'stalled', 'resilience requires self-recovery')",
            [],
        )
        .unwrap();
        let row = query_rows(conn, "SELECT first_principles FROM decision_log LIMIT 1", [])
            .unwrap();
        assert!(row[0]["first_principles"].as_str().unwrap().contains("resilience"));
    }

    #[test]
    fn decision_log_query_by_plan_id_filters_correctly() {
        let db = setup_db();
        let conn = db.connection();
        for i in 0..3i64 {
            conn.execute(
                "INSERT INTO decision_log (plan_id, decision, reasoning) VALUES (?1, ?2, 'r')",
                rusqlite::params![if i < 2 { 724i64 } else { 725i64 }, format!("d{i}")],
            )
            .unwrap();
        }
        let rows = query_rows(conn, "SELECT id FROM decision_log WHERE plan_id = 724", [])
            .unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn decision_log_outcome_nullable() {
        let db = setup_db();
        let conn = db.connection();
        conn.execute("INSERT INTO decision_log (decision, reasoning) VALUES ('d', 'r')", [])
            .unwrap();
        let row = query_rows(conn, "SELECT outcome FROM decision_log LIMIT 1", []).unwrap();
        assert!(row[0].get("outcome").map_or(true, |v| v.is_null()));
    }
}
