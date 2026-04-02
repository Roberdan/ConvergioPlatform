// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// POST /api/kernel/ask — free-form question to local kernel model.
// Routes via voice_router: simple → Qwen local, "ali/opus" → Ali (Opus cloud).

#[cfg(feature = "kernel")]
pub use inner::*;

#[cfg(feature = "kernel")]
mod inner {
    use crate::kernel::api::handlers::KernelState;
    use crate::kernel::voice_router::{classify_intent, route_intent, VoiceIntent};
    use axum::extract::State;
    use axum::Json;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize)]
    pub struct AskRequest {
        pub question: String,
    }

    #[derive(Debug, Serialize)]
    pub struct AskResponse {
        pub answer: String,
    }

    /// POST /api/kernel/ask
    ///
    /// Routes through voice_router: classifies intent first.
    /// - Simple queries (stato, costi) → handled by voice_router directly
    /// - EscalateToAli / write intents → cloud escalation (Opus)
    pub async fn handle_ask(
        State(state): State<KernelState>,
        Json(body): Json<AskRequest>,
    ) -> Json<AskResponse> {
        let question = body.question.clone();
        let engine = state.engine.clone();

        // Check inference level — cloud for write intents or Ali escalation
        let level = {
            let eng = engine.lock().unwrap_or_else(|p| p.into_inner());
            eng.inference_level_for(&question)
        };

        let answer = if level == crate::kernel::engine::InferenceLevel::Cloud {
            crate::kernel::cloud_escalation::cloud_ask_with_tools(&question, "").await
        } else {
            let q = question.clone();
            let q2 = question.clone();
            // Classify locally; if EscalateToAli, escalate to cloud
            let intent = {
                let eng = engine.lock().unwrap_or_else(|p| p.into_inner());
                classify_intent(&q, &eng)
            };
            if matches!(intent, VoiceIntent::EscalateToAli { .. }) {
                crate::kernel::cloud_escalation::cloud_ask_with_tools(&q2, "").await
            } else {
                tokio::task::spawn_blocking(move || {
                    let du = std::env::var("DAEMON_URL")
                        .unwrap_or_else(|_| "http://localhost:8420".into());
                    route_intent(intent, &du)
                })
                .await
                .unwrap_or_else(|e| format!("Errore interno: {e}"))
            }
        };

        Json(AskResponse { answer })
    }
}
