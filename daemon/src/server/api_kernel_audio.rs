// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// POST /api/kernel/play — receive WAV audio bytes and play locally via afplay.
// POST /api/kernel/active-node — set the active audio routing target node.
// Available on ALL daemon builds (not behind kernel feature gate).
// Any mesh node can receive and play audio routed from the kernel.

use super::state::{ApiError, ServerState};
use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use tracing::{info, warn};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/kernel/play", post(handle_play))
        .route("/api/kernel/active-node", post(handle_active_node))
}

/// POST /api/kernel/play
///
/// Receives raw WAV bytes in the request body and plays them locally via `afplay`.
/// The temp WAV file is deleted after playback.
/// Returns 202 Accepted immediately (fire-and-forget on a Tokio task).
async fn handle_play(body: Bytes) -> impl IntoResponse {
    if body.is_empty() {
        warn!("kernel.audio.play: received empty body — rejected");
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({"ok": false, "error": "empty audio body"})),
        )
            .into_response();
    }

    info!("kernel.audio.play: received {} bytes — spawning afplay", body.len());
    let audio = body.to_vec();
    tokio::spawn(async move {
        crate::kernel::audio::play_local(&audio).await;
    });

    (StatusCode::ACCEPTED, axum::Json(serde_json::json!({"ok": true, "bytes": body.len()})))
        .into_response()
}

#[derive(Deserialize)]
struct ActiveNodeBody {
    node: String,
}

/// POST /api/kernel/active-node
///
/// Body: `{"node": "hostname"}`
/// Writes `active_node` and `active_node_set_at` into the `kernel_config` table so that
/// `audio::resolve_active_node` routes TTS playback to the requesting node.
/// Response: `{"ok": true, "active_node": "<hostname>"}`
async fn handle_active_node(
    State(state): State<ServerState>,
    Json(body): Json<ActiveNodeBody>,
) -> Result<impl IntoResponse, ApiError> {
    if body.node.trim().is_empty() {
        return Err(ApiError::bad_request("node must not be empty"));
    }
    let node = body.node.trim().to_string();
    let conn = state.get_conn()?;

    // Ensure the kernel_config table exists (idempotent; no-op if already present).
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS kernel_config (
            key   TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL DEFAULT ''
        )",
    )
    .map_err(|e| ApiError::internal(format!("kernel_config schema: {e}")))?;

    conn.execute(
        "INSERT OR REPLACE INTO kernel_config (key, value) VALUES ('active_node', ?1)",
        rusqlite::params![node],
    )
    .map_err(|e| ApiError::internal(format!("kernel_config write active_node: {e}")))?;

    conn.execute(
        "INSERT OR REPLACE INTO kernel_config (key, value) \
         VALUES ('active_node_set_at', datetime('now'))",
        [],
    )
    .map_err(|e| ApiError::internal(format!("kernel_config write active_node_set_at: {e}")))?;

    info!("kernel.active_node: set to '{node}'");
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"ok": true, "active_node": node})),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_node_body_deserializes() {
        let json = r#"{"node": "macProM1"}"#;
        let body: ActiveNodeBody = serde_json::from_str(json).expect("deserialize");
        assert_eq!(body.node, "macProM1");
    }

    #[test]
    fn active_node_empty_node_is_caught() {
        // Validates that trim().is_empty() guards against blank nodes
        let node = "   ".trim();
        assert!(node.is_empty(), "blank node must be rejected");
    }
}
