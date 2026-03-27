// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// POST /api/kernel/ask — free-form question to local kernel model.
// Returns plain human-readable text in the `answer` field.

#[cfg(feature = "kernel")]
pub use inner::*;

#[cfg(feature = "kernel")]
mod inner {
    use crate::kernel::api::handlers::KernelState;
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
    /// Forwards the question to the local kernel model (`engine.ask`).
    /// Returns plain Italian text — suitable for TTS rendering.
    pub async fn handle_ask(
        State(state): State<KernelState>,
        Json(body): Json<AskRequest>,
    ) -> Json<AskResponse> {
        let answer = {
            let engine = state.engine.lock().unwrap_or_else(|p| p.into_inner());
            engine.ask(&body.question)
        };
        Json(AskResponse { answer })
    }
}
