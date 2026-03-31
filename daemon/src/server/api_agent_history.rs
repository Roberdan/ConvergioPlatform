// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// GET /api/agents/history — paginated, filtered agent activity history.

use super::state::{query_rows, ApiError, ServerState};
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct HistoryParams {
    /// ISO datetime lower bound (default: 30 days ago)
    pub since: Option<String>,
    /// ISO datetime upper bound (default: now)
    pub until: Option<String>,
    /// Filter by status (running, completed, failed)
    pub status: Option<String>,
    /// Filter by model name
    pub model: Option<String>,
    /// Filter by agent_id (exact or prefix match)
    pub agent_id: Option<String>,
    /// Max rows returned (default 50, max 500)
    pub limit: Option<u32>,
}

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/agents/history", get(api_agent_history))
}

async fn api_agent_history(
    State(state): State<ServerState>,
    Query(params): Query<HistoryParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit.unwrap_or(50).min(500);
    let conn = state.get_conn()?;

    // Build WHERE clauses and collect params as owned strings
    let mut conditions: Vec<String> = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();

    // Default since: 30 days ago
    let since = params
        .since
        .unwrap_or_else(|| "30_days_ago".to_string());
    if since == "30_days_ago" {
        conditions.push("started_at >= datetime('now', '-30 days')".to_string());
    } else {
        conditions.push(format!("started_at >= ?{}", bind_values.len() + 1));
        bind_values.push(since);
    }

    if let Some(until) = params.until {
        conditions.push(format!("started_at <= ?{}", bind_values.len() + 1));
        bind_values.push(until);
    }

    if let Some(status) = params.status {
        conditions.push(format!("status = ?{}", bind_values.len() + 1));
        bind_values.push(status);
    }

    if let Some(model) = params.model {
        conditions.push(format!("model = ?{}", bind_values.len() + 1));
        bind_values.push(model);
    }

    if let Some(agent_id) = params.agent_id {
        conditions.push(format!("agent_id LIKE ?{}", bind_values.len() + 1));
        bind_values.push(format!("{agent_id}%"));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT agent_id, agent_type, model, status, description, \
         task_db_id, plan_id, tokens_in, tokens_out, cost_usd, \
         started_at, completed_at, duration_s, host \
         FROM agent_activity {where_clause} \
         ORDER BY started_at DESC LIMIT {limit}"
    );

    let params_ref: Vec<&dyn rusqlite::types::ToSql> = bind_values
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();

    let rows = query_rows(&conn, &sql, params_ref.as_slice())
        .map_err(|e| ApiError::internal(format!("history query failed: {e}")))?;

    Ok(Json(Value::Array(rows)))
}
