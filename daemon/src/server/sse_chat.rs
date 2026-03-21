// Real LLM streaming for chat SSE endpoint.
// Reads session + message history from DB, streams via llm_client,
// saves assistant reply and upserts agent_activity for budget tracking.

use super::llm_client::{self, ChatMessage, Provider, StreamChunk};
use super::state::{ApiError, ServerState};
use axum::extract::{Path, State};
use axum::response::sse::{Event, Sse};
use futures_util::StreamExt;
use serde_json::json;
use std::convert::Infallible;
use tokio_stream::wrappers::ReceiverStream;
use tracing::warn;

/// Determine LLM provider from model name.
/// SECURITY: Always route through LiteLLM proxy — never call Anthropic API directly.
/// LiteLLM handles auth, rate limiting, cost tracking, and supports all providers.
fn provider_for_model(_model: &str) -> Provider {
    Provider::LiteLLM
}

/// Cost estimate per 1k tokens (rough defaults for budget visibility).
fn estimate_cost(model: &str, input_tokens: u64, output_tokens: u64) -> f64 {
    let (in_rate, out_rate) = if model.contains("opus") {
        (0.015, 0.075)
    } else if model.contains("sonnet") {
        (0.003, 0.015)
    } else if model.contains("haiku") {
        (0.00025, 0.00125)
    } else {
        (0.002, 0.01)
    };
    (input_tokens as f64 * in_rate + output_tokens as f64 * out_rate) / 1000.0
}

pub async fn chat_stream_sse(
    State(state): State<ServerState>,
    Path(session_id): Path<String>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let conn = state.get_conn()?;
    ensure_chat_tables(&conn)?;

    // Read session metadata for model preference
    let model = read_session_model(&conn, &session_id)?;
    let provider = provider_for_model(&model);

    // Build message history from chat_messages
    let messages = read_message_history(&conn, &session_id)?;
    if messages.is_empty() {
        return Err(ApiError::bad_request("no messages in session"));
    }

    // Spawn LLM stream and relay as SSE events
    let llm_stream = llm_client::stream_chat(provider, &model, messages);
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);
    let model_clone = model.clone();
    let sid_clone = session_id.clone();
    let state_clone = state.clone();

    tokio::spawn(async move {
        relay_llm_to_sse(llm_stream, tx, state_clone, sid_clone, model_clone).await;
    });

    Ok(Sse::new(ReceiverStream::new(rx)))
}

/// Read model preference from session metadata_json, fallback to default.
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
                if !m.is_empty() {
                    return Ok(m.to_string());
                }
            }
        }
    }
    Ok("claude-sonnet-4-20250514".to_string())
}

/// Load message history for context window.
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
            Ok(ChatMessage {
                role: row.get(0)?,
                content: row.get(1)?,
            })
        })
        .map_err(|e| ApiError::internal(format!("query messages: {e}")))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| ApiError::internal(format!("read messages: {e}")))
}

/// Relay LLM stream chunks to SSE events, then persist results.
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
                    json!({"type": "usage", "input_tokens": total_in, "output_tokens": total_out, "cost": cost}).to_string(),
                )
            }
            StreamChunk::Error(msg) => {
                warn!("LLM stream error for session {session_id}: {msg}");
                let _ = tx
                    .send(Ok(Event::default()
                        .event("chat")
                        .data(json!({"type": "error", "message": msg}).to_string())))
                    .await;
                break;
            }
        };
        if tx.send(Ok(event)).await.is_err() {
            break; // client disconnected
        }
    }

    // Persist assistant message and update budget tracking
    let cost = estimate_cost(&model, total_in, total_out);
    if !full_text.is_empty() {
        if let Ok(conn) = state.get_conn() {
            save_assistant_message(&conn, &session_id, &full_text, &model, total_in, total_out);
            upsert_agent_activity(&conn, &session_id, &model, total_in, total_out, cost);
        }
    }

    // Send final done event
    let _ = tx
        .send(Ok(Event::default()
            .event("chat")
            .data(json!({"type": "done"}).to_string())))
        .await;
}

/// Save the assistant reply into chat_messages and update session timestamp.
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

/// Upsert agent_activity with chat token counts and cost for budget visibility.
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

/// Ensure chat tables exist (idempotent).
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
