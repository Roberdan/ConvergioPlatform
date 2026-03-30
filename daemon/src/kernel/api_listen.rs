// Kernel API listen endpoint — mic record + transcribe.

#[cfg(feature = "kernel")]
pub mod listen {
    use crate::kernel::api::handlers::KernelState;
    use crate::kernel::stt::TranscribeResponse;
    use axum::extract::State;
    use axum::http::{header, StatusCode};
    use axum::response::IntoResponse;
    use axum::Json;

    /// POST /api/kernel/listen — record mic, transcribe, return text.
    pub async fn handle_listen(State(state): State<KernelState>) -> impl IntoResponse {
        // Shell out to `rec` (SoX) to capture microphone input.
        let wav_path = "/tmp/cvg_listen.wav";
        let rec_result = std::process::Command::new("rec")
            .args([
                "-q", "-r", "16000", "-c", "1", wav_path,
                "silence", "1", "0.1", "1%", "1", "1.5", "1%",
            ])
            .status();

        match rec_result {
            Ok(s) if s.success() => {}
            Ok(s) => {
                tracing::warn!(code = ?s.code(), "rec exited non-zero");
                if let Err(e) = std::fs::remove_file(wav_path) {
                    tracing::debug!(path = wav_path, error = %e, "cleanup wav after rec failure");
                }
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "application/json")],
                    br#"{"error":"mic recording failed"}"#.to_vec(),
                )
                    .into_response();
            }
            Err(e) => {
                tracing::warn!(error = %e, "rec (SoX) not available");
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    [(header::CONTENT_TYPE, "application/json")],
                    format!(r#"{{"error":"rec not available: {e}"}}"#).into_bytes(),
                )
                    .into_response();
            }
        }

        let audio = std::fs::read(wav_path).unwrap_or_default();
        if let Err(e) = std::fs::remove_file(wav_path) {
            tracing::debug!(path = wav_path, error = %e, "privacy cleanup: delete wav after transcribe");
        }
        let result = {
            let stt = state.stt.lock().unwrap_or_else(|p| p.into_inner());
            stt.transcribe(&audio)
        };
        match result {
            Ok(t) => (StatusCode::OK, Json(TranscribeResponse::from(t))).into_response(),
            Err(e) => {
                tracing::warn!(error = %e, "stt transcription after listen failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "application/json")],
                    format!(r#"{{"error":"stt failed: {e}"}}"#).into_bytes(),
                )
                    .into_response()
            }
        }
    }
}
