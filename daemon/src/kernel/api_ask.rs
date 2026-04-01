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
    /// - EscalateToAli → forwards to Ali (Opus) via chat API
    /// - EscalateToAli (unrecognized) → forwarded to Ali (Opus)
    pub async fn handle_ask(
        State(state): State<KernelState>,
        Json(body): Json<AskRequest>,
    ) -> Json<AskResponse> {
        let question = body.question.clone();
        let engine = state.engine.clone();
        let answer = tokio::task::spawn_blocking(move || {
            let q = question.to_lowercase();
            // Check for Ali escalation FIRST (before any LLM classification)
            if q.starts_with("ali ") || q.starts_with("ali,") || q == "ali"
                || q.contains(" ali ") || q.contains("chiedi ad ali")
                || q.contains("opus") || q.contains("cloud")
            {
                return route_intent(
                    VoiceIntent::EscalateToAli { question: question.clone() },
                    "http://localhost:8420",
                );
            }
            // For everything else: classify intent then route
            let eng = engine.lock().unwrap_or_else(|p| p.into_inner());
            let intent = classify_intent(&question, &eng);
            match &intent {
                VoiceIntent::EscalateToAli { .. } => eng.ask(&question),
                _ => route_intent(intent, "http://localhost:8420"),
            }
        })
        .await
        .unwrap_or_else(|e| format!("Errore interno: {e}"));

        Json(AskResponse { answer })
    }
}
