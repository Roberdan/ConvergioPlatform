// Workspace event recording endpoint — POST /api/workspace/events/record.
// Why: hook scripts (PostToolUse) need a fire-and-forget path to log file ops
// without coupling to workspace management logic in api_workspace.rs.
use super::state::{ApiError, ServerState};
use crate::workspace::events::WorkspaceAction;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use rusqlite::params;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct RecordEventRequest {
    pub workspace_id: String,
    pub agent: String,
    pub action: String,
    pub file_path: Option<String>,
    pub detail: Option<String>,
    pub metadata: Option<String>,
}

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/workspace/events/record", post(record_event))
}

// POST /api/workspace/events/record
// Accepts file operation events from hook scripts; returns {ok, event_id}.
// Action strings are stored verbatim — unknown values are valid for extensibility.
// Daemon-down callers use --connect-timeout 1 || true; no workspace existence check.
async fn record_event(
    State(state): State<ServerState>,
    Json(body): Json<RecordEventRequest>,
) -> Result<Json<Value>, ApiError> {
    // Normalise known action names to their canonical form; pass unknown strings through.
    let action_str = body
        .action
        .parse::<WorkspaceAction>()
        .map(|a| a.to_string())
        .unwrap_or(body.action.clone());

    let conn = state.get_conn()?;

    conn.execute(
        "INSERT INTO workspace_events \
         (workspace_id, agent, action, file_path, detail, metadata) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            body.workspace_id,
            body.agent,
            action_str,
            body.file_path,
            body.detail,
            body.metadata
        ],
    )
    .map_err(|e| ApiError::internal(format!("event insert failed: {e}")))?;

    let event_id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .map_err(|e| ApiError::internal(format!("rowid query failed: {e}")))?;

    Ok(Json(json!({"ok": true, "event_id": event_id})))
}
