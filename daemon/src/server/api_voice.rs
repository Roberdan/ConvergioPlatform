// Voice pipeline API: start, stop, status, test.
// When the `voice` feature is enabled, endpoints manage a real VoicePipeline.
// Without the feature, endpoints return safe stubs so the CLI always works.

use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use std::sync::Mutex;

use super::state::ServerState;

#[cfg(feature = "voice")]
use crate::voice::{VoiceConfig, VoicePipeline, VoiceState};

/// Shared pipeline state behind a Mutex.
/// Without the `voice` feature we track only an idle/listening flag.
#[cfg(feature = "voice")]
static PIPELINE: std::sync::LazyLock<Mutex<VoicePipeline>> =
    std::sync::LazyLock::new(|| Mutex::new(VoicePipeline::new(VoiceConfig::default())));

#[cfg(not(feature = "voice"))]
static VOICE_ACTIVE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/voice/status", get(handle_status))
        .route("/api/voice/start", post(handle_start))
        .route("/api/voice/stop", post(handle_stop))
        .route("/api/voice/test", post(handle_test))
}

/// GET /api/voice/status — current pipeline state and config.
async fn handle_status() -> impl IntoResponse {
    #[cfg(feature = "voice")]
    {
        let pipeline = PIPELINE.lock().expect("voice pipeline lock");
        let state = pipeline.state();
        let config = pipeline.config().clone();
        Json(json!({
            "state": state.to_string(),
            "config": config,
            "feature_enabled": true,
        }))
    }
    #[cfg(not(feature = "voice"))]
    {
        let active = VOICE_ACTIVE.load(std::sync::atomic::Ordering::Relaxed);
        let state = if active { "listening" } else { "idle" };
        Json(json!({
            "state": state,
            "config": {
                "wake_word": "convergio",
                "whisper_model": "small",
                "vad_threshold": 0.5,
                "tts_voice": "default",
                "tts_rate": 1.0,
                "prefer_local": true,
            },
            "feature_enabled": false,
        }))
    }
}

/// POST /api/voice/start — activate the pipeline.
async fn handle_start() -> impl IntoResponse {
    #[cfg(feature = "voice")]
    {
        let mut pipeline = PIPELINE.lock().expect("voice pipeline lock");
        match pipeline.start() {
            Ok(()) => Json(json!({
                "ok": true,
                "state": pipeline.state().to_string(),
            })),
            Err(e) => Json(json!({
                "ok": false,
                "error": e.to_string(),
            })),
        }
    }
    #[cfg(not(feature = "voice"))]
    {
        VOICE_ACTIVE.store(true, std::sync::atomic::Ordering::Relaxed);
        Json(json!({
            "ok": true,
            "state": "listening",
            "note": "voice feature not compiled; stub mode",
        }))
    }
}

/// POST /api/voice/stop — deactivate the pipeline.
async fn handle_stop() -> impl IntoResponse {
    #[cfg(feature = "voice")]
    {
        let mut pipeline = PIPELINE.lock().expect("voice pipeline lock");
        pipeline.stop();
        Json(json!({
            "ok": true,
            "state": pipeline.state().to_string(),
        }))
    }
    #[cfg(not(feature = "voice"))]
    {
        VOICE_ACTIVE.store(false, std::sync::atomic::Ordering::Relaxed);
        Json(json!({
            "ok": true,
            "state": "idle",
        }))
    }
}

/// POST /api/voice/test — quick audio subsystem check.
async fn handle_test() -> impl IntoResponse {
    #[cfg(feature = "voice")]
    {
        // Verify pipeline can start/stop without errors.
        let mut pipeline = PIPELINE.lock().expect("voice pipeline lock");
        let prev = pipeline.state();
        let ok = pipeline.start().is_ok();
        pipeline.stop();
        // Restore previous state if it was active.
        if prev == VoiceState::Listening {
            let _ = pipeline.start();
        }
        Json(json!({
            "ok": ok,
            "test": "pipeline_start_stop",
            "feature_enabled": true,
        }))
    }
    #[cfg(not(feature = "voice"))]
    {
        Json(json!({
            "ok": true,
            "test": "stub_mode",
            "feature_enabled": false,
        }))
    }
}
