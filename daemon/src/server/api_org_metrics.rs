use crate::server::api_ipc::ensure_ipc_schema;
use crate::server::state::{query_rows, ApiError, ServerState};
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct MetricsQuery {
    pub period: Option<String>,
}

/// Convert period string to SQL datetime offset.
fn period_offset(period: &str) -> Result<&'static str, ApiError> {
    match period {
        "day" => Ok("-1 day"),
        "week" => Ok("-7 days"),
        "month" => Ok("-30 days"),
        _ => Err(ApiError::bad_request("period must be day, week, or month")),
    }
}

/// GET /api/orgs/:slug/metrics?period=day|week|month
pub async fn get_metrics(
    State(state): State<ServerState>,
    Path(slug): Path<String>,
    Query(q): Query<MetricsQuery>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let period = q.period.unwrap_or_else(|| "day".to_string());
    let offset = period_offset(&period)?;

    // Task counts from plans belonging to this org
    let task_sql = format!(
        "SELECT COALESCE(SUM(tasks_done), 0) AS tasks_completed,
                COALESCE(SUM(tasks_total), 0) AS tasks_total
         FROM plans WHERE org_id = ?1
           AND created_at >= datetime('now', '{offset}')"
    );
    let task_rows = query_rows(&conn, &task_sql, rusqlite::params![slug])?;
    let task_row = task_rows.first().cloned().unwrap_or(json!({}));
    let tasks_completed = task_row.get("tasks_completed").and_then(|v| v.as_i64()).unwrap_or(0);
    let tasks_total = task_row.get("tasks_total").and_then(|v| v.as_i64()).unwrap_or(0);

    // Tokens spent and cost via agent_activity joined through ipc_org_members
    let token_sql = format!(
        "SELECT COALESCE(SUM(a.tokens_total), 0) AS tokens_spent,
                COALESCE(SUM(a.cost_usd), 0.0) AS cost_usd
         FROM agent_activity a
         JOIN ipc_org_members m ON m.agent = a.agent_id
         WHERE m.org_id = ?1
           AND a.started_at >= datetime('now', '{offset}')"
    );
    let token_rows = query_rows(&conn, &token_sql, rusqlite::params![slug])?;
    let token_row = token_rows.first().cloned().unwrap_or(json!({}));
    let tokens_spent = token_row.get("tokens_spent").and_then(|v| v.as_i64()).unwrap_or(0);
    let cost_usd = token_row.get("cost_usd").and_then(|v| v.as_f64()).unwrap_or(0.0);

    // Avg task duration and success rate from tasks in org plans
    let dur_sql = format!(
        "SELECT AVG(CASE WHEN t.validated_at IS NOT NULL AND t.started_at IS NOT NULL
                    THEN (julianday(t.validated_at) - julianday(t.started_at)) * 86400
                    ELSE NULL END) AS avg_duration_s,
                SUM(CASE WHEN t.status = 'done' THEN 1 ELSE 0 END) AS done_count,
                SUM(CASE WHEN t.status = 'failed' THEN 1 ELSE 0 END) AS failed_count
         FROM tasks t
         JOIN plans p ON p.id = t.plan_id
         WHERE p.org_id = ?1
           AND t.started_at >= datetime('now', '{offset}')"
    );
    let dur_rows = query_rows(&conn, &dur_sql, rusqlite::params![slug])?;
    let dur_row = dur_rows.first().cloned().unwrap_or(json!({}));
    let avg_duration_s = dur_row.get("avg_duration_s").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let done_count = dur_row.get("done_count").and_then(|v| v.as_i64()).unwrap_or(0);
    let failed_count = dur_row.get("failed_count").and_then(|v| v.as_i64()).unwrap_or(0);
    let total_resolved = done_count + failed_count;
    let success_rate = if total_resolved > 0 {
        (done_count as f64) / (total_resolved as f64)
    } else {
        0.0
    };

    Ok(Json(json!({
        "ok": true,
        "metrics": {
            "tasks_completed": tasks_completed,
            "tasks_total": tasks_total,
            "tokens_spent": tokens_spent,
            "avg_duration_s": (avg_duration_s * 100.0).round() / 100.0,
            "success_rate": (success_rate * 10000.0).round() / 10000.0,
            "cost_usd": (cost_usd * 100.0).round() / 100.0,
        },
        "period": period,
    })))
}

/// GET /api/orgs/:slug/report — combined metrics + decisions + events
pub async fn get_report(
    State(state): State<ServerState>,
    Path(slug): Path<String>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;

    // Reuse metrics logic with day period
    let metrics_resp = get_metrics(
        State(state.clone()),
        Path(slug.clone()),
        Query(MetricsQuery { period: Some("day".to_string()) }),
    )
    .await?;
    let metrics = metrics_resp.0.get("metrics").cloned().unwrap_or(json!({}));

    // Recent decisions
    let decisions = query_rows(
        &conn,
        "SELECT decision, rationale, decided_by, created_at
         FROM ipc_decisions WHERE org_id = ?1
         ORDER BY created_at DESC LIMIT 5",
        rusqlite::params![slug],
    )?;

    // Recent events (table may not exist yet — W2-T1 adds it)
    let events = query_rows(
        &conn,
        "SELECT id, event_type, agent_id, description, created_at
         FROM ipc_org_events WHERE org_id = ?1
         ORDER BY created_at DESC LIMIT 10",
        rusqlite::params![slug],
    )
    .unwrap_or_default();

    Ok(Json(json!({
        "ok": true,
        "report": {
            "metrics": metrics,
            "decisions": decisions,
            "events": events,
        }
    })))
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/orgs/:slug/metrics", get(get_metrics))
        .route("/api/orgs/:slug/report", get(get_report))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_period_offset_valid() {
        assert_eq!(period_offset("day").unwrap(), "-1 day");
        assert_eq!(period_offset("week").unwrap(), "-7 days");
        assert_eq!(period_offset("month").unwrap(), "-30 days");
    }

    #[test]
    fn test_period_offset_invalid() {
        assert!(period_offset("year").is_err());
        assert!(period_offset("").is_err());
    }

    #[test]
    fn test_success_rate_calculation() {
        // done=8, failed=2 => 0.8
        let done = 8_i64;
        let failed = 2_i64;
        let total = done + failed;
        let rate = if total > 0 {
            (done as f64) / (total as f64)
        } else {
            0.0
        };
        assert!((rate - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_success_rate_zero_tasks() {
        let done = 0_i64;
        let failed = 0_i64;
        let total = done + failed;
        let rate = if total > 0 {
            (done as f64) / (total as f64)
        } else {
            0.0
        };
        assert!((rate - 0.0).abs() < f64::EPSILON);
    }
}
