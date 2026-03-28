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
    /// - AskAli (unrecognized) → Qwen local with context stuffing
    pub async fn handle_ask(
        State(state): State<KernelState>,
        Json(body): Json<AskRequest>,
    ) -> Json<AskResponse> {
        let question = body.question.clone();
        let engine = state.engine.clone();
        // Route through voice_router (uses spawn_blocking for reqwest::blocking)
        let answer = tokio::task::spawn_blocking(move || {
            let eng = engine.lock().unwrap_or_else(|p| p.into_inner());
            let intent = classify_intent(&question, &eng);
            match &intent {
                // AskAli → use engine.ask() with context stuffing (Qwen reasons on data)
                VoiceIntent::AskAli { .. } => eng.ask(&question),
                // Everything else (StatusCheck, CostQuery, EscalateToAli, etc.) → voice_router
                _ => route_intent(intent, "http://localhost:8420"),
            }
        })
        .await
        .unwrap_or_else(|e| format!("Errore interno: {e}"));

        Json(AskResponse { answer })
    }
}
