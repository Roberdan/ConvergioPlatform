// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Kernel API endpoints — gated behind the "kernel" feature flag.
// POST /api/kernel/classify and GET /api/kernel/status.

#[cfg(feature = "kernel")]
pub mod handlers {
    use crate::kernel::engine::{KernelConfig, KernelEngine, KernelStatus};
    use axum::extract::State;
    use axum::routing::{get, post};
    use axum::{Json, Router};
    use serde::{Deserialize, Serialize};
    use std::sync::{Arc, Mutex};
    use std::time::SystemTime;

    /// Shared kernel state wired into the Axum router.
    #[derive(Clone)]
    pub struct KernelState {
        pub engine: Arc<Mutex<KernelEngine>>,
    }

    impl KernelState {
        pub fn new(config: KernelConfig) -> Self {
            let mut engine = KernelEngine::new(config.clone());
            // Pre-load the default model on startup.
            engine.load_model(&config.default_model);
            Self {
                engine: Arc::new(Mutex::new(engine)),
            }
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
        pub models_loaded: u32,
        pub ram_gb: f64,
        pub uptime_secs: u64,
        pub active_node: Option<String>,
        pub last_check: Option<String>,
    }

    impl From<KernelStatus> for StatusResponse {
        fn from(s: KernelStatus) -> Self {
            Self {
                models_loaded: s.models_loaded,
                ram_gb: s.ram_gb,
                uptime_secs: s.uptime_secs,
                active_node: s.active_node,
                last_check: s.last_check,
            }
        }
    }

    pub fn router() -> Router<KernelState> {
        Router::new()
            .route("/api/kernel/classify", post(handle_classify))
            .route("/api/kernel/status", get(handle_status))
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

    /// Timestamp helper — ISO 8601 UTC.
    #[allow(dead_code)]
    fn now_iso() -> String {
        let secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // Minimal ISO 8601 without chrono dependency (seconds granularity).
        let epoch_offset = secs;
        format!("{epoch_offset}")
    }
}
