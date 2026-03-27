// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// POST /api/kernel/play — receive WAV audio bytes and play locally via afplay.
// Available on ALL daemon builds (not behind kernel feature gate).
// Any mesh node can receive and play audio routed from the kernel.

use super::state::{ApiError, ServerState};
use axum::body::Bytes;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;
use tracing::{info, warn};

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/kernel/play", post(handle_play))
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
