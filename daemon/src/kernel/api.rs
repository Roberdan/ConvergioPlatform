// Kernel API endpoints — gated behind "kernel" feature flag.

#[cfg(feature = "kernel")]
pub mod handlers {
    use crate::kernel::engine::{KernelConfig, KernelEngine, KernelStatus};
    use crate::kernel::stt::{SttEngine, TranscribeResponse};
    use crate::kernel::tts::TtsEngine;
    use axum::body::Bytes;
    use axum::extract::State;
    use axum::http::{header, StatusCode};
    use axum::response::IntoResponse;
    use axum::routing::{get, post};
    use axum::{Json, Router};
    use serde::{Deserialize, Serialize};
    use std::sync::{Arc, Mutex};
    use std::time::SystemTime;

    /// Shared kernel state wired into the Axum router.
    #[derive(Clone)]
    pub struct KernelState {
        pub engine: Arc<Mutex<KernelEngine>>,
        pub tts: Arc<Mutex<TtsEngine>>,
        pub stt: Arc<Mutex<SttEngine>>,
    }

    impl KernelState {
        pub fn new(config: KernelConfig) -> Self {
            let mut engine = KernelEngine::new(config.clone());
            engine.load_model(&config.default_model);
            let mut stt = SttEngine::new();
            if stt.is_available() {
                stt.load();
            }
            let state = Self {
                engine: Arc::new(Mutex::new(engine)),
                tts: Arc::new(Mutex::new(TtsEngine::new())),
                stt: Arc::new(Mutex::new(stt)),
            };

            // Spawn Telegram polling loop if token is configured
            if let (Ok(token), Ok(chat_id_str)) = (
                std::env::var("CONVERGIO_TELEGRAM_TOKEN"),
                std::env::var("CONVERGIO_TELEGRAM_CHAT_ID"),
            ) {
                let chat_id: i64 = chat_id_str.parse().unwrap_or(0);
                let engine_clone = Arc::new(KernelEngine::new(config.clone()));
                let daemon_url = std::env::var("DAEMON_URL")
                    .unwrap_or_else(|_| "http://localhost:8420".to_string());
                tracing::info!("jarvis: spawning Telegram poll loop for chat_id={chat_id}");
                crate::kernel::telegram_poll::spawn_telegram_poll(
                    token, chat_id, daemon_url, engine_clone,
                );
            } else {
                tracing::info!("jarvis: Telegram not configured (no CONVERGIO_TELEGRAM_TOKEN)");
            }

            state
        }
    }

    #[derive(Debug, Deserialize)]
    pub struct ClassifyRequest {
        pub situation: String,
    }

    #[derive(Debug, Serialize)]
    pub struct ClassifyResponse {
        pub severity: String,
        pub action: String,
        pub reason: String,
    }

    #[derive(Debug, Serialize)]
    pub struct StatusResponse {
        pub name: String,
        pub models_loaded: u32,
        pub ram_gb: f64,
        pub uptime_secs: u64,
        pub active_node: Option<String>,
        pub last_check: Option<String>,
    }

    impl From<KernelStatus> for StatusResponse {
        fn from(s: KernelStatus) -> Self {
            Self {
                name: "Jarvis".to_string(),
                models_loaded: s.models_loaded,
                ram_gb: s.ram_gb,
                uptime_secs: s.uptime_secs,
                active_node: s.active_node,
                last_check: s.last_check,
            }
        }
    }

    #[derive(Debug, Deserialize)]
    pub struct SpeakRequest {
        pub text: String,
        /// BCP-47 locale, e.g. "it-IT" or "en-US".
        pub locale: String,
    }

    pub fn router() -> Router<KernelState> {
        Router::new()
            .route("/api/kernel/classify", post(handle_classify))
            .route("/api/kernel/status", get(handle_status))
            .route("/api/kernel/speak", post(handle_speak))
            .route("/api/kernel/transcribe", post(handle_transcribe))
            .route("/api/kernel/listen", post(handle_listen))
            .route("/api/kernel/ask", post(crate::kernel::api_ask::handle_ask))
    }

    async fn handle_classify(
        State(state): State<KernelState>,
        Json(body): Json<ClassifyRequest>,
    ) -> Json<ClassifyResponse> {
        let action = {
            let engine = state.engine.lock().unwrap_or_else(|p| p.into_inner());
            engine.classify(&body.situation)
        };
        Json(ClassifyResponse {
            severity: format!("{:?}", action.severity).to_lowercase(),
            action: action.action,
            reason: action.reason,
        })
    }

    async fn handle_status(State(state): State<KernelState>) -> Json<StatusResponse> {
        let status = {
            let engine = state.engine.lock().unwrap_or_else(|p| p.into_inner());
            engine.status()
        };
        Json(StatusResponse::from(status))
    }

    /// POST /api/kernel/speak — text to WAV audio bytes.
    async fn handle_speak(
        State(state): State<KernelState>,
        Json(body): Json<SpeakRequest>,
    ) -> impl IntoResponse {
        let result = {
            let mut tts = state.tts.lock().unwrap_or_else(|p| p.into_inner());
            tts.speak(&body.text, &body.locale)
        };
        match result {
            Ok(wav) => (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "audio/wav")],
                wav,
            )
                .into_response(),
            Err(e) => {
                tracing::warn!(error = %e, text = %body.text, "tts synthesis failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "application/json")],
                    format!(r#"{{"error":"tts failed: {e}"}}"#).into_bytes(),
                )
                    .into_response()
            }
        }
    }

    /// POST /api/kernel/transcribe — audio bytes to text.
    async fn handle_transcribe(
        State(state): State<KernelState>,
        body: Bytes,
    ) -> impl IntoResponse {
        let result = {
            let stt = state.stt.lock().unwrap_or_else(|p| p.into_inner());
            stt.transcribe(&body)
        };
        match result {
            Ok(t) => (StatusCode::OK, Json(TranscribeResponse::from(t))).into_response(),
            Err(e) => {
                tracing::warn!(error = %e, "stt transcription failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "application/json")],
                    format!(r#"{{"error":"stt failed: {e}"}}"#).into_bytes(),
                )
                    .into_response()
            }
        }
    }

    /// POST /api/kernel/listen — record mic, transcribe, return text.
    async fn handle_listen(State(state): State<KernelState>) -> impl IntoResponse {
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
                let _ = std::fs::remove_file(wav_path);
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
        let _ = std::fs::remove_file(wav_path); // privacy: delete immediately
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

    /// Timestamp helper — Unix epoch seconds (no chrono dependency).
    #[allow(dead_code)]
    fn now_iso() -> String {
        let secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("{secs}")
    }
}
