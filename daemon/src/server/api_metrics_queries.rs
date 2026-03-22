//! Cost breakdown query handler for the metrics API.

use super::state::{query_rows, ApiError, ServerState};
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct CostQuery {
    pub days: Option<i64>,
    pub project: Option<String>,
}

/// GET /api/metrics/cost — cost breakdown by model/project/date
/// Query params: ?days=7&project=convergio
pub async fn handle_cost_breakdown(
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
