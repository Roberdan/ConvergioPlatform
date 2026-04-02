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
            // Cloud path: use Opus via stream_with_fallback
            crate::kernel::cloud_escalation::cloud_ask_with_tools(&question, "").await
        } else {
            // Local path: classify + route
            let q = question.clone();
            tokio::task::spawn_blocking(move || {
                let eng = engine.lock().unwrap_or_else(|p| p.into_inner());
                let intent = classify_intent(&q, &eng);
                match &intent {
                    VoiceIntent::EscalateToAli { .. } => eng.ask(&q),
                    _ => route_intent(intent, "http://localhost:8420"),
                }
            })
            .await
            .unwrap_or_else(|e| format!("Errore interno: {e}"))
        };

        Json(AskResponse { answer })
    }
}
