// Real LLM streaming for chat SSE endpoint.
// Routes through provider.rs (tier routing + fallback chains).
// Saves assistant reply and upserts agent_activity for budget tracking.

use super::llm_client::{self, ChatMessage, StreamChunk};
use super::provider::{estimate_cost, provider_for_model, stream_with_fallback};
use super::state::{ApiError, ServerState};
use crate::ipc::budget;
use chrono::Local;
use axum::extract::{Path, State};
use axum::response::sse::{Event, Sse};
use futures_util::StreamExt;
use serde_json::json;
use std::convert::Infallible;
use tokio_stream::wrappers::ReceiverStream;
use tracing::warn;

pub async fn chat_stream_sse(
    State(state): State<ServerState>,
    Path(session_id): Path<String>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let conn = state.get_conn()?;
    ensure_chat_tables(&conn)?;

    let model = read_session_model(&conn, &session_id)?;
    let (provider, resolved_model) = provider_for_model(&model);
    let messages = read_message_history(&conn, &session_id)?;
    if messages.is_empty() {
        return Err(ApiError::bad_request("no messages in session"));
    }

    let llm_stream = stream_with_fallback(provider, &resolved_model, messages);
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);
    let model_clone = resolved_model.clone();
    let sid_clone = session_id.clone();
    let state_clone = state.clone();

    tokio::spawn(async move {
        relay_llm_to_sse(llm_stream, tx, state_clone, sid_clone, model_clone).await;
    });

    Ok(Sse::new(ReceiverStream::new(rx)))
}

fn read_session_model(conn: &rusqlite::Connection, session_id: &str) -> Result<String, ApiError> {
    let meta: Option<String> = conn
        .query_row(
            "SELECT metadata_json FROM chat_sessions WHERE id=?1",
            rusqlite::params![session_id],
            |row| row.get(0),
        )
        .map_err(|_| ApiError::bad_request("session not found"))?;

    if let Some(ref json_str) = meta {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(m) = parsed.get("model").and_then(|v| v.as_str()) {
                if !m.is_empty() { return Ok(m.to_string()); }
            }
        }
    }
    Ok("claude-sonnet-4-20250514".to_string())
}

fn read_message_history(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<Vec<ChatMessage>, ApiError> {
    let mut stmt = conn
        .prepare(
            "SELECT role, content FROM chat_messages \
             WHERE session_id=?1 ORDER BY id ASC",
        )
        .map_err(|e| ApiError::internal(format!("prepare messages: {e}")))?;
    let rows = stmt
        .query_map(rusqlite::params![session_id], |row| {
            Ok(ChatMessage { role: row.get(0)?, content: row.get(1)? })
        })
        .map_err(|e| ApiError::internal(format!("query messages: {e}")))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| ApiError::internal(format!("read messages: {e}")))
}

async fn relay_llm_to_sse(
    mut llm_stream: llm_client::ChatStream,
    tx: tokio::sync::mpsc::Sender<Result<Event, Infallible>>,
    state: ServerState,
    session_id: String,
    model: String,
) {
    let mut full_text = String::new();
    let mut total_in: u64 = 0;
    let mut total_out: u64 = 0;

    while let Some(chunk) = llm_stream.next().await {
        let event = match chunk {
            StreamChunk::Text(text) => {
                full_text.push_str(&text);
                Event::default()
                    .event("chat")
                    .data(json!({"type": "token", "content": text}).to_string())
            }
            StreamChunk::Usage(usage) => {
                total_in += usage.input_tokens;
                total_out += usage.output_tokens;
                let cost = estimate_cost(&model, total_in, total_out);
                Event::default().event("chat").data(
                    json!({"type": "usage", "input_tokens": total_in,
                           "output_tokens": total_out, "cost": cost}).to_string(),
                )
            }
            StreamChunk::Error(msg) => {
                warn!("LLM stream error for session {session_id}: {msg}");
                if let Err(e) = tx
                    .send(Ok(Event::default()
                        .event("chat")
                        .data(json!({"type": "error", "message": msg}).to_string())))
                    .await
                {
                    warn!("chat error event send failed: {e}");
                }
                break;
            }
        };
        if tx.send(Ok(event)).await.is_err() {
            break;
        }
    }

    let cost = estimate_cost(&model, total_in, total_out);
    if !full_text.is_empty() {
        if let Ok(conn) = state.get_conn() {
            save_assistant_message(&conn, &session_id, &full_text, &model, total_in, total_out);
            upsert_agent_activity(&conn, &session_id, &model, total_in, total_out, cost);
            log_budget_usage(&conn, &session_id, &model, total_in, total_out, cost);
        }
    }

    if let Err(e) = tx
        .send(Ok(Event::default()
            .event("chat")
            .data(json!({"type": "done"}).to_string())))
        .await
    {
        tracing::debug!("chat done event send (client disconnected): {e}");
    }
}

fn save_assistant_message(
    conn: &rusqlite::Connection,
    session_id: &str,
    content: &str,
    model: &str,
    tokens_in: u64,
    tokens_out: u64,
) {
    if let Err(e) = conn.execute(
        "INSERT INTO chat_messages(session_id,role,content,model,tokens_in,tokens_out) \
         VALUES(?1,'assistant',?2,?3,?4,?5)",
        rusqlite::params![session_id, content, model, tokens_in, tokens_out],
    ) {
        warn!("Failed to save assistant message: {e}");
    }
    if let Err(e) = conn.execute(
        "UPDATE chat_sessions SET last_message_at=CURRENT_TIMESTAMP, \
         updated_at=CURRENT_TIMESTAMP WHERE id=?1",
        rusqlite::params![session_id],
    ) {
        warn!("Failed to update session timestamp: {e}");
    }
}

fn upsert_agent_activity(
    conn: &rusqlite::Connection,
    session_id: &str,
    model: &str,
    tokens_in: u64,
    tokens_out: u64,
    cost: f64,
) {
    let agent_id = format!("chat-{session_id}");
    let tokens_total = tokens_in + tokens_out;
    if let Err(e) = conn.execute(
        "INSERT INTO agent_activity(agent_id, agent_type, model, description, status, \
         tokens_in, tokens_out, tokens_total, cost_usd, started_at, completed_at, \
         parent_session, region) \
         VALUES(?1,'chat',?2,'Chat session',?3,?4,?5,?6,?7,datetime('now'),datetime('now'),?8,'chat') \
         ON CONFLICT(agent_id) DO UPDATE SET \
         tokens_in=agent_activity.tokens_in+excluded.tokens_in, \
         tokens_out=agent_activity.tokens_out+excluded.tokens_out, \
         tokens_total=agent_activity.tokens_total+excluded.tokens_total, \
         cost_usd=agent_activity.cost_usd+excluded.cost_usd, \
         completed_at=excluded.completed_at, \
         status=excluded.status, \
         model=excluded.model",
        rusqlite::params![agent_id, model, "completed", tokens_in, tokens_out, tokens_total, cost, session_id],
    ) {
        warn!("Failed to upsert agent_activity for chat: {e}");
    }
}

fn log_budget_usage(
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

fn ensure_chat_tables(conn: &rusqlite::Connection) -> Result<(), ApiError> {
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
