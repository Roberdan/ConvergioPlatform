// Budget logging and chat schema helpers for SSE chat endpoint.

use crate::ipc::budget;
use tracing::warn;

use super::state::ApiError;

pub fn log_budget_usage(
    conn: &rusqlite::Connection,
    session_id: &str,
    model: &str,
    tokens_in: u64,
    tokens_out: u64,
    cost: f64,
) {
    // Ensure table exists — budget module may not have run its own init yet.
    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS ipc_budget_log \
         (id INTEGER PRIMARY KEY, subscription TEXT, date TEXT, \
          tokens_in INTEGER, tokens_out INTEGER, estimated_cost_usd REAL, \
          model TEXT, task_ref TEXT);",
    );
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let entry = budget::BudgetEntry {
        subscription: "default".to_string(),
        date,
        tokens_in: tokens_in as i64,
        tokens_out: tokens_out as i64,
        estimated_cost_usd: cost,
        model: model.to_string(),
        task_ref: session_id.to_string(),
    };
    if let Err(e) = budget::log_usage(conn, &entry) {
        warn!("budget log_usage failed: {e}");
        return;
    }
    // Ensure subscriptions table for threshold check
    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS ipc_subscriptions \
         (name TEXT PRIMARY KEY, provider TEXT, plan TEXT, \
          budget_usd REAL, reset_day INTEGER, models TEXT);",
    );
    match budget::check_budget_thresholds(conn, "default") {
        Ok(Some(alert)) => warn!("Budget threshold hit: {}", alert.message),
        Ok(None) => {}
        Err(e) => warn!("budget threshold check failed: {e}"),
    }
}

pub fn ensure_chat_tables(conn: &rusqlite::Connection) -> Result<(), ApiError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS chat_sessions (id TEXT PRIMARY KEY, project_id INTEGER, \
         plan_id INTEGER, task_db_id INTEGER, title TEXT NOT NULL, \
         status TEXT NOT NULL DEFAULT 'active', metadata_json TEXT, \
         created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, \
         updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, last_message_at TEXT);\
         CREATE TABLE IF NOT EXISTS chat_messages (id INTEGER PRIMARY KEY AUTOINCREMENT, \
         session_id TEXT NOT NULL, role TEXT NOT NULL, content TEXT NOT NULL, \
         requirement_id INTEGER, model TEXT, tokens_in INTEGER DEFAULT 0, \
         tokens_out INTEGER DEFAULT 0, metadata_json TEXT, \
         created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);",
    )
    .map_err(|e| ApiError::internal(format!("chat schema: {e}")))
}
