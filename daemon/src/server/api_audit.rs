// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// GET /api/audit/project/:project_id — aggregated project audit report.
// Joins solve_sessions, plans, tasks, execution_runs, agent_activity,
// deliverables, and knowledge_base for a single project.

use super::state::{query_rows, ApiError, ServerState};
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/audit/project/:project_id", get(project_audit))
}

async fn project_audit(
    State(state): State<ServerState>,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;

    // 1. solve_sessions for this project
    let solve_sessions = query_rows_safe(
        &conn,
        "solve_sessions",
        "SELECT id, timestamp, triage_level, routed_to, plan_id \
         FROM solve_sessions WHERE project_id = ?1 ORDER BY id DESC",
        &project_id,
    )?;

    // 2. plans + tasks (status, model, effort, timeline)
    let plans = query_rows_safe(
        &conn,
        "plans",
        "SELECT id, name, status, tasks_total, tasks_done, created_at, updated_at \
         FROM plans WHERE project_id = ?1 ORDER BY id DESC",
        &project_id,
    )?;

    let plan_ids = plans
        .iter()
        .filter_map(|p| p.get("id").and_then(Value::as_i64))
        .collect::<Vec<_>>();

    let tasks = if plan_ids.is_empty() {
        vec![]
    } else {
        let placeholders = plan_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT id, plan_id, task_id, title, status, model, effort, wave_id_fk \
             FROM tasks WHERE plan_id IN ({placeholders}) ORDER BY plan_id, id"
        );
        let params = plan_ids
            .iter()
            .map(|id| *id as i64)
            .collect::<Vec<i64>>();
        query_rows_dynamic(&conn, &sql, &params)?
    };

    // 3. execution_runs (cost, duration, model usage)
    let runs = if plan_ids.is_empty() {
        vec![]
    } else {
        let placeholders = plan_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT id, goal, status, plan_id, cost_usd, duration_s, \
             model, agents_used, started_at, ended_at \
             FROM execution_runs WHERE plan_id IN ({placeholders}) ORDER BY id DESC"
        );
        let params = plan_ids.iter().map(|id| *id as i64).collect::<Vec<i64>>();
        query_rows_dynamic(&conn, &sql, &params)?
    };

    // 4. agent_activity for plan IDs in this project
    let agent_activity = if plan_ids.is_empty() {
        vec![]
    } else {
        let placeholders = plan_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT agent_id, action, status, model, cost_usd, duration_s, \
             started_at, completed_at \
             FROM agent_activity WHERE plan_id IN ({placeholders}) ORDER BY agent_id"
        );
        let params = plan_ids.iter().map(|id| *id as i64).collect::<Vec<i64>>();
        query_rows_dynamic(&conn, &sql, &params)?
    };

    // 5. deliverables (status, approval state)
    let deliverables = query_rows_safe(
        &conn,
        "deliverables",
        "SELECT id, name, output_type, status, version, approved_by, approved_at, \
         created_at FROM deliverables WHERE project_id = ?1 ORDER BY id DESC",
        &project_id,
    )?;

    // 6. knowledge_base learnings tagged with project
    let kb_learnings = query_rows_safe(
        &conn,
        "knowledge_base",
        "SELECT id, domain, title, content, created_at \
         FROM knowledge_base WHERE domain = ?1 OR title LIKE '%' || ?1 || '%' \
         ORDER BY id DESC LIMIT 50",
        &project_id,
    )?;

    // Summary stats
    let total_cost: f64 = runs
        .iter()
        .filter_map(|r| r.get("cost_usd").and_then(Value::as_f64))
        .sum();
    let task_done = tasks
        .iter()
        .filter(|t| t.get("status").and_then(Value::as_str) == Some("done"))
        .count();

    let report = json!({
        "project_id": project_id,
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "summary": {
            "solve_sessions": solve_sessions.len(),
            "plans": plans.len(),
            "tasks_total": tasks.len(),
            "tasks_done": task_done,
            "runs": runs.len(),
            "total_cost_usd": total_cost,
            "deliverables": deliverables.len(),
            "kb_learnings": kb_learnings.len(),
        },
        "solve_sessions": solve_sessions,
        "plans": plans,
        "tasks": tasks,
        "execution_runs": runs,
        "agent_activity": agent_activity,
        "deliverables": deliverables,
        "kb_learnings": kb_learnings,
    });

    Ok(Json(report))
}

/// Query rows, returning empty vec if the table does not exist.
fn query_rows_safe(
    conn: &r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>,
    table: &str,
    sql: &str,
    project_id: &str,
) -> Result<Vec<Value>, ApiError> {
    if !table_exists(conn, table) {
        return Ok(vec![]);
    }
    query_rows(conn, sql, rusqlite::params![project_id])
}

/// Query with dynamic IN-clause params.
fn query_rows_dynamic(
    conn: &r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>,
    sql: &str,
    params: &[i64],
) -> Result<Vec<Value>, ApiError> {
    let boxed: Vec<Box<dyn rusqlite::types::ToSql>> =
        params.iter().map(|p| Box::new(*p) as Box<dyn rusqlite::types::ToSql>).collect();
    let refs: Vec<&dyn rusqlite::types::ToSql> = boxed.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| ApiError::internal(format!("prepare failed: {e}")))?;
    let column_names: Vec<String> = stmt
        .column_names()
        .iter()
        .map(|c| c.to_string())
        .collect();
    let rows = stmt
        .query_map(refs.as_slice(), |row| row_to_json(row, &column_names))
        .map_err(|e| ApiError::internal(format!("query failed: {e}")))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| ApiError::internal(format!("row decode failed: {e}")))
}

fn table_exists(
    conn: &r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>,
    name: &str,
) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        rusqlite::params![name],
        |row| row.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
}

fn row_to_json(
    row: &rusqlite::Row<'_>,
    columns: &[String],
) -> rusqlite::Result<Value> {
    let mut obj = serde_json::Map::new();
    for (idx, col) in columns.iter().enumerate() {
        let val = match row.get_ref(idx)? {
            rusqlite::types::ValueRef::Null => Value::Null,
            rusqlite::types::ValueRef::Integer(v) => Value::from(v),
            rusqlite::types::ValueRef::Real(v) => Value::from(v),
            rusqlite::types::ValueRef::Text(v) => {
                Value::from(String::from_utf8_lossy(v).to_string())
            }
            rusqlite::types::ValueRef::Blob(v) => Value::from(format!("blob:{}", v.len())),
        };
        obj.insert(col.clone(), val);
    }
    Ok(Value::Object(obj))
}
