// A2UI SSE stream + TTL cleanup background task.

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::unfold;
use rusqlite::params;
use serde_json::json;
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;

use super::state::ServerState;

pub async fn a2ui_stream(
    State(state): State<ServerState>,
) -> Sse<impl futures_util::stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.ws_tx.subscribe();
    let stream = unfold(rx, |mut rx| async move {
        let next = loop {
            match rx.recv().await {
                Ok(val) => {
                    let t = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    match t {
                        "a2ui_block" => {
                            let data = serde_json::to_string(&val).unwrap_or_default();
                            break Ok(Event::default().event("block").data(data));
                        }
                        "a2ui_dismiss" => {
                            let data = serde_json::to_string(&val).unwrap_or_default();
                            break Ok(Event::default().event("dismiss").data(data));
                        }
                        _ => {}
                    }
                }
                Err(RecvError::Lagged(_)) => {
                    break Ok(Event::default()
                        .event("reconnect")
                        .data("{\"reconnect\":true}"));
                }
                Err(RecvError::Closed) => return None,
            }
        };
        Some((next, rx))
    });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

pub async fn ttl_cleanup_loop(state: ServerState) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        let conn = match state.get_conn() {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut expired: Vec<String> = Vec::new();
        if let Ok(mut stmt) = conn.prepare(
            "SELECT id FROM a2ui_blocks \
             WHERE status = 'active' AND ttl_seconds IS NOT NULL \
             AND (julianday('now') - julianday(created_at)) * 86400 > ttl_seconds",
        ) {
            if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                for id in rows.flatten() {
                    expired.push(id);
                }
            }
        }
        for id in &expired {
            let _ = conn.execute(
                "UPDATE a2ui_blocks SET status = 'expired' WHERE id = ?1",
                params![id],
            );
            let _ = state.ws_tx.send(json!({"type": "a2ui_dismiss", "block_id": id}));
        }
    }
}
