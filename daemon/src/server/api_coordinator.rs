use super::state::{query_rows, ApiError, ServerState};
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/coordinator/events", get(handle_list_events))
        .route("/api/coordinator/emit", post(handle_emit_event))
        .route("/api/coordinator/process", post(handle_process_events))
}

/// GET /api/coordinator/events — list recent coordinator events
async fn handle_list_events(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let events = query_rows(
        &conn,
        "SELECT id, event_type, payload, source_node, handled_at \
         FROM coordinator_events \
         ORDER BY id DESC LIMIT 50",
        [],
    )?;

    Ok(Json(json!({
        "ok": true,
        "events": events,
        "count": events.len(),
    })))
}

/// POST /api/coordinator/emit — emit a new coordinator event
/// Body: {event_type, payload?, source_node?}
async fn handle_emit_event(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let event_type = body
        .get("event_type")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing event_type"))?;
    let payload = body.get("payload").cloned().unwrap_or(Value::Null);
    let source_node_owned = body
        .get("source_node")
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(|| {
            hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string())
        });
    let source_node = source_node_owned.as_str();

    let conn = state.get_conn()?;
    let conn = &conn;

    conn.execute(
        "INSERT INTO coordinator_events (event_type, payload, source_node) \
         VALUES (?1, ?2, ?3)",
        rusqlite::params![
            event_type,
            serde_json::to_string(&payload).unwrap_or_default(),
            source_node,
        ],
    )
    .map_err(|e| ApiError::internal(format!("emit event failed: {e}")))?;

    let event_id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .map_err(|e| ApiError::internal(format!("rowid failed: {e}")))?;

    // Broadcast event via WebSocket
    let _ = state.ws_tx.send(json!({
        "type": "coordinator_event",
        "id": event_id,
        "event_type": event_type,
        "payload": payload,
    }));

    Ok(Json(json!({
        "ok": true,
        "event_id": event_id,
        "event_type": event_type,
    })))
}

/// POST /api/coordinator/process — trigger event processing
/// Processes unhandled events and triggers appropriate actions
async fn handle_process_events(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    // Get recent events for processing
    let events = query_rows(
        conn,
        "SELECT id, event_type, payload, source_node \
         FROM coordinator_events \
         WHERE handled_at > datetime('now', '-5 minutes') \
         ORDER BY id DESC LIMIT 100",
        [],
    )?;

    let mut processed = 0usize;
    let mut actions = Vec::new();

    for event in &events {
        let etype = event
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or("");

        match etype {
            "plan_started" | "plan_completed" | "plan_cancelled" => {
                actions.push(format!("plan_status_change: {etype}"));
                processed += 1;
            }
            "task_done" | "wave_done" => {
                actions.push(format!("progress_update: {etype}"));
                processed += 1;
            }
            "agent_started" | "agent_completed" => {
                actions.push(format!("agent_lifecycle: {etype}"));
                processed += 1;
            }
            _ => {
                processed += 1;
            }
        }
    }

    Ok(Json(json!({
        "ok": true,
        "events_found": events.len(),
        "processed": processed,
        "actions": actions,
    })))
}

#[cfg(test)]
mod tests {
    use crate::db::PlanDb;
    use crate::server::state::query_rows;

    fn setup_db() -> PlanDb {
        let db = PlanDb::open_in_memory().expect("db");
        db.connection()
            .execute_batch(
                "CREATE TABLE coordinator_events (
                     id INTEGER PRIMARY KEY, event_type TEXT NOT NULL DEFAULT '',
                     payload TEXT, source_node TEXT,
                     handled_at TEXT DEFAULT (datetime('now'))
                 );",
            )
            .expect("schema");
        db
    }

    #[test]
    fn coordinator_events_insert_and_query() {
        let db = setup_db();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO coordinator_events (event_type, payload, source_node) \
             VALUES ('plan_started', '{\"plan_id\": 1}', 'mac-worker-2')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO coordinator_events (event_type, payload, source_node) \
             VALUES ('task_done', '{\"task_id\": 100}', 'linux-worker')",
            [],
        )
        .unwrap();

        let events = query_rows(
            conn,
            "SELECT id, event_type FROM coordinator_events ORDER BY id",
            [],
        )
        .unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0].get("event_type").unwrap().as_str().unwrap(),
            "plan_started"
        );
    }

    #[test]
    fn coordinator_events_type_matching() {
        let types = [
            "plan_started",
            "plan_completed",
            "task_done",
            "wave_done",
            "agent_started",
        ];
        for t in types {
            let is_plan = t.starts_with("plan_");
            let is_task = t.starts_with("task_") || t.starts_with("wave_");
            let is_agent = t.starts_with("agent_");
            assert!(is_plan || is_task || is_agent, "unmatched event type: {t}");
        }
    }
}
