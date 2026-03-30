// Budget status endpoint — aggregates ipc_budget_log for the dashboard.
use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/budget/status", get(handle_budget_status))
}

fn ensure_budget_tables(conn: &rusqlite::Connection) {
    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS ipc_budget_log \
         (id INTEGER PRIMARY KEY, subscription TEXT, date TEXT, \
          tokens_in INTEGER, tokens_out INTEGER, estimated_cost_usd REAL, \
          model TEXT, task_ref TEXT);\
         CREATE TABLE IF NOT EXISTS ipc_subscriptions \
         (name TEXT PRIMARY KEY, provider TEXT, plan TEXT, \
          budget_usd REAL, reset_day INTEGER, models TEXT);",
    );
}

async fn handle_budget_status(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    ensure_budget_tables(&conn);

    let used_today: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(estimated_cost_usd),0.0) FROM ipc_budget_log \
             WHERE date=date('now')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    let used_month: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(estimated_cost_usd),0.0) FROM ipc_budget_log \
             WHERE date>=date('now','start of month')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    let limit: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(budget_usd),0.0) FROM ipc_subscriptions",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    // Project monthly spend: (used_month / elapsed_days) * 30
    let day_of_month: i64 = conn
        .query_row(
            "SELECT CAST(strftime('%d','now') AS INTEGER)",
            [],
            |r| r.get(0),
        )
        .unwrap_or(1);
    let elapsed = day_of_month.max(1) as f64;
    let projected_month = (used_month / elapsed) * 30.0;
    let projected_overage = (projected_month - limit).max(0.0);

    let mut by_model: HashMap<String, Value> = HashMap::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT model, SUM(tokens_in), SUM(tokens_out), SUM(estimated_cost_usd) \
         FROM ipc_budget_log GROUP BY model ORDER BY SUM(estimated_cost_usd) DESC",
    ) {
        if let Ok(rows) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1).unwrap_or(0),
                row.get::<_, i64>(2).unwrap_or(0),
                row.get::<_, f64>(3).unwrap_or(0.0),
            ))
        }) {
            for row in rows.flatten() {
                let (model, tokens_in, tokens_out, cost) = row;
                by_model.insert(
                    model,
                    json!({"tokens_in": tokens_in, "tokens_out": tokens_out, "cost": cost}),
                );
            }
        }
    }

    Ok(Json(json!({
        "used_today": used_today,
        "used_month": used_month,
        "limit": limit,
        "projected_overage": projected_overage,
        "by_model": by_model,
    })))
}
