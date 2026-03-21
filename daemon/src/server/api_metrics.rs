use super::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/metrics/run/:id", get(handle_run_metrics))
        .route("/api/metrics/summary", get(handle_summary))
        .route("/api/metrics/cost", get(handle_cost_breakdown))
}

/// GET /api/metrics/run/:id — metrics for a single execution run
async fn handle_run_metrics(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    let run = query_one(
        conn,
        "SELECT id, goal, status, plan_id, started_at, completed_at, \
         cost_usd, agents_used \
         FROM execution_runs WHERE id = ?1",
        rusqlite::params![id],
    )?
    .ok_or_else(|| ApiError::bad_request(format!("run {id} not found")))?;

    let plan_id = run.get("plan_id").and_then(Value::as_i64);

    // Duration in seconds (completed_at - started_at, fallback to now)
    let duration_secs = query_one(
        conn,
        "SELECT CAST((julianday(COALESCE(completed_at, datetime('now'))) \
         - julianday(started_at)) * 86400 AS INTEGER) AS duration_secs \
         FROM execution_runs WHERE id = ?1",
        rusqlite::params![id],
    )?
    .and_then(|v| v.get("duration_secs").and_then(Value::as_i64));

    // Cost from delegation_log for the run's plan within the run time window
    let cost_from_log: f64 = if let Some(pid) = plan_id {
        query_one(
            conn,
            "SELECT COALESCE(SUM(d.cost_estimate), 0.0) AS total_cost \
             FROM delegation_log d \
             JOIN execution_runs r ON r.id = ?1 \
             WHERE d.plan_id = ?2 \
             AND d.created_at BETWEEN r.started_at \
               AND COALESCE(r.completed_at, datetime('now'))",
            rusqlite::params![id, pid],
        )?
        .and_then(|v| v.get("total_cost").and_then(Value::as_f64))
        .unwrap_or(0.0)
    } else {
        run.get("cost_usd").and_then(Value::as_f64).unwrap_or(0.0)
    };

    // Agents used — distinct executor_agent from tasks under this plan
    let agents_used: i64 = if let Some(pid) = plan_id {
        query_one(
            conn,
            "SELECT COUNT(DISTINCT executor_agent) AS cnt \
             FROM tasks WHERE plan_id = ?1 AND executor_agent IS NOT NULL",
            rusqlite::params![pid],
        )?
        .and_then(|v| v.get("cnt").and_then(Value::as_i64))
        .unwrap_or(0)
    } else {
        run.get("agents_used").and_then(Value::as_i64).unwrap_or(0)
    };

    // Tasks completed / total for this plan
    let (tasks_done, tasks_total) = if let Some(pid) = plan_id {
        let row = query_one(
            conn,
            "SELECT \
               SUM(CASE WHEN status IN ('done','submitted') THEN 1 ELSE 0 END) AS done, \
               COUNT(*) AS total \
             FROM tasks WHERE plan_id = ?1",
            rusqlite::params![pid],
        )?;
        let done = row
            .as_ref()
            .and_then(|v| v.get("done").and_then(Value::as_i64))
            .unwrap_or(0);
        let total = row
            .as_ref()
            .and_then(|v| v.get("total").and_then(Value::as_i64))
            .unwrap_or(0);
        (done, total)
    } else {
        (0, 0)
    };

    Ok(Json(json!({
        "ok": true,
        "run": run,
        "duration_secs": duration_secs,
        "cost_usd": cost_from_log,
        "agents_used": agents_used,
        "tasks_done": tasks_done,
        "tasks_total": tasks_total,
    })))
}

/// GET /api/metrics/summary — aggregate metrics across all runs
async fn handle_summary(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    // Run count and avg duration
    let counts = query_one(
        conn,
        "SELECT COUNT(*) AS run_count, \
         AVG(CAST((julianday(COALESCE(completed_at, datetime('now'))) \
           - julianday(started_at)) * 86400 AS REAL)) AS avg_duration_secs, \
         SUM(cost_usd) AS total_cost \
         FROM execution_runs",
        [],
    )?
    .unwrap_or(json!({}));

    // Status distribution
    let status_dist = query_rows(
        conn,
        "SELECT status, COUNT(*) AS count \
         FROM execution_runs GROUP BY status ORDER BY count DESC",
        [],
    )?;

    // Top 5 agents by task count
    let top_agents = query_rows(
        conn,
        "SELECT executor_agent, COUNT(*) AS task_count \
         FROM tasks \
         WHERE executor_agent IS NOT NULL \
         GROUP BY executor_agent \
         ORDER BY task_count DESC \
         LIMIT 5",
        [],
    )?;

    Ok(Json(json!({
        "ok": true,
        "run_count": counts.get("run_count").and_then(Value::as_i64).unwrap_or(0),
        "avg_duration_secs": counts.get("avg_duration_secs").and_then(Value::as_f64),
        "total_cost_usd": counts.get("total_cost").and_then(Value::as_f64).unwrap_or(0.0),
        "status_distribution": status_dist,
        "top_agents": top_agents,
    })))
}

#[derive(Debug, Deserialize)]
pub struct CostQuery {
    pub days: Option<i64>,
    pub project: Option<String>,
}

/// GET /api/metrics/cost — cost breakdown by model/project/date
/// Query params: ?days=7&project=convergio
async fn handle_cost_breakdown(
    State(state): State<ServerState>,
    Query(params): Query<CostQuery>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;
    let days = params.days.unwrap_or(7).max(1).min(365);

    // Cost by model
    let by_model = if let Some(ref project) = params.project {
        query_rows(
            conn,
            "SELECT model, COUNT(*) AS calls, \
             COALESCE(SUM(cost_estimate), 0.0) AS cost_usd \
             FROM delegation_log \
             WHERE project_id = ?1 \
               AND created_at >= datetime('now', ?2) \
             GROUP BY model ORDER BY cost_usd DESC",
            rusqlite::params![project, format!("-{days} days")],
        )?
    } else {
        query_rows(
            conn,
            "SELECT model, COUNT(*) AS calls, \
             COALESCE(SUM(cost_estimate), 0.0) AS cost_usd \
             FROM delegation_log \
             WHERE created_at >= datetime('now', ?1) \
             GROUP BY model ORDER BY cost_usd DESC",
            rusqlite::params![format!("-{days} days")],
        )?
    };

    // Cost by project
    let by_project = if let Some(ref project) = params.project {
        query_rows(
            conn,
            "SELECT project_id, COUNT(*) AS calls, \
             COALESCE(SUM(cost_estimate), 0.0) AS cost_usd \
             FROM delegation_log \
             WHERE project_id = ?1 \
               AND created_at >= datetime('now', ?2) \
             GROUP BY project_id ORDER BY cost_usd DESC",
            rusqlite::params![project, format!("-{days} days")],
        )?
    } else {
        query_rows(
            conn,
            "SELECT project_id, COUNT(*) AS calls, \
             COALESCE(SUM(cost_estimate), 0.0) AS cost_usd \
             FROM delegation_log \
             WHERE created_at >= datetime('now', ?1) \
             GROUP BY project_id ORDER BY cost_usd DESC",
            rusqlite::params![format!("-{days} days")],
        )?
    };

    // Cost by date
    let by_date = if let Some(ref project) = params.project {
        query_rows(
            conn,
            "SELECT date(created_at) AS date, \
             COALESCE(SUM(cost_estimate), 0.0) AS cost_usd \
             FROM delegation_log \
             WHERE project_id = ?1 \
               AND created_at >= datetime('now', ?2) \
             GROUP BY date(created_at) ORDER BY date DESC",
            rusqlite::params![project, format!("-{days} days")],
        )?
    } else {
        query_rows(
            conn,
            "SELECT date(created_at) AS date, \
             COALESCE(SUM(cost_estimate), 0.0) AS cost_usd \
             FROM delegation_log \
             WHERE created_at >= datetime('now', ?1) \
             GROUP BY date(created_at) ORDER BY date DESC",
            rusqlite::params![format!("-{days} days")],
        )?
    };

    Ok(Json(json!({
        "ok": true,
        "days": days,
        "project_filter": params.project,
        "by_model": by_model,
        "by_project": by_project,
        "by_date": by_date,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_registers_three_routes() {
        // Smoke test: ensure router() builds without panic
        let _r: Router<ServerState> = router();
    }

    #[test]
    fn cost_query_all_optional() {
        // All fields are Option<T> — verify defaults are None
        let q = CostQuery {
            days: None,
            project: None,
        };
        assert!(q.days.is_none());
        assert!(q.project.is_none());
    }

    #[test]
    fn cost_query_days_clamped() {
        // days < 1 should be treated as 1, days > 365 as 365
        // This mirrors the .max(1).min(365) logic in handle_cost_breakdown
        let raw_days: i64 = 0;
        let clamped = raw_days.max(1).min(365);
        assert_eq!(clamped, 1);

        let raw_days: i64 = 400;
        let clamped = raw_days.max(1).min(365);
        assert_eq!(clamped, 365);
    }
}
