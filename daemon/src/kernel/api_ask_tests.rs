// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for POST /api/kernel/ask handler.

#[cfg(test)]
#[cfg(feature = "kernel")]
mod tests {
    use crate::kernel::api::handlers::KernelState;
    use crate::kernel::api_ask::{AskRequest, AskResponse, handle_ask};
    use crate::kernel::engine::{KernelConfig, KernelEngine};
    use crate::kernel::stt::SttEngine;
    use crate::kernel::tts::TtsEngine;
    use axum::Json;
    use axum::extract::State;
    use std::sync::{Arc, Mutex};

    fn make_state() -> KernelState {
        let config = KernelConfig::default();
        let engine = KernelEngine::new(config.clone());
        KernelState {
            engine: Arc::new(Mutex::new(engine)),
            tts: Arc::new(Mutex::new(TtsEngine::new())),
            stt: Arc::new(Mutex::new(SttEngine::new())),
        }
    }

    #[tokio::test]
    async fn test_handle_ask_returns_nonempty_answer() {
        // GIVEN a KernelState with no model loaded
        let state = make_state();
        let req = AskRequest { question: "ciao".to_string() };

        // WHEN we call handle_ask
        let Json(resp): Json<AskResponse> = handle_ask(State(state), Json(req)).await;

        // THEN answer is non-empty (fallback text when model unavailable)
        assert!(!resp.answer.is_empty(), "answer must not be empty");
    }

    #[tokio::test]
    async fn test_handle_ask_answer_is_string() {
        let state = make_state();
        let req = AskRequest { question: "stato del sistema".to_string() };

        let Json(resp): Json<AskResponse> = handle_ask(State(state), Json(req)).await;

        // answer field is a plain string, not JSON
        assert!(
            !resp.answer.starts_with('{') && !resp.answer.starts_with('['),
            "answer must be plain text, not JSON: {}", resp.answer
        );
    }

    #[tokio::test]
    async fn test_ask_request_deserialization() {
        let json = r#"{"question":"test domanda"}"#;
        let req: AskRequest = serde_json::from_str(json).expect("must deserialize");
        assert_eq!(req.question, "test domanda");
    }

    #[tokio::test]
    async fn test_ask_response_serialization() {
        let resp = AskResponse { answer: "risposta di test".to_string() };
        let json = serde_json::to_string(&resp).expect("must serialize");
        assert!(json.contains("answer"));
        assert!(json.contains("risposta di test"));
    }
}
