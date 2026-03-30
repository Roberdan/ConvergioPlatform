// GET /api/agents/profiles — list all agent sandbox profiles.

use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/agents/profiles", get(list_profiles))
}

async fn list_profiles(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;

    // Ensure tables exist (first-access safety — idempotent)
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS agent_profiles (\
         id INTEGER PRIMARY KEY, \
         name TEXT UNIQUE NOT NULL, \
         filesystem_allowlist TEXT, \
         network_allowlist TEXT, \
         allowed_commands TEXT, \
         created_at TEXT DEFAULT (datetime('now'))\
         )",
    )
    .map_err(|e| ApiError::internal(format!("migration: {e}")))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, name, filesystem_allowlist, network_allowlist, \
             allowed_commands, created_at \
             FROM agent_profiles ORDER BY name",
        )
        .map_err(|e| ApiError::internal(format!("prepare: {e}")))?;

    let profiles: Vec<Value> = stmt
        .query_map([], |row| {
            Ok(json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "filesystem_allowlist": row.get::<_, Option<String>>(2)?,
                "network_allowlist": row.get::<_, Option<String>>(3)?,
                "allowed_commands": row.get::<_, Option<String>>(4)?,
                "created_at": row.get::<_, Option<String>>(5)?,
            }))
        })
        .map_err(|e| ApiError::internal(format!("query: {e}")))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| ApiError::internal(format!("row: {e}")))?;

    Ok(Json(json!({ "profiles": profiles, "total": profiles.len() })))
}
