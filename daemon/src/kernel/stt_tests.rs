// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Unit tests for kernel::stt — Whisper STT engine.

use crate::kernel::stt::{parse_locale_from_stderr, SttEngine, SttError, Transcription, TranscribeResponse};
use std::time::Duration;

// --- SttEngine construction ---

#[test]
fn test_stt_engine_default_model_name() {
    let engine = SttEngine::new();
    assert_eq!(engine.model_name, "whisper-small");
}

#[test]
fn test_stt_engine_not_loaded_by_default() {
    let engine = SttEngine::new();
    assert!(!engine.loaded);
}

#[test]
fn test_stt_engine_load_sets_flag() {
    let mut engine = SttEngine::new();
    engine.load();
    assert!(engine.loaded);
}

// --- transcribe guard: not loaded ---

#[test]
fn test_transcribe_errors_when_not_loaded() {
    let engine = SttEngine::new();
    let result = engine.transcribe(b"fake audio");
    assert!(result.is_err());
    match result.unwrap_err() {
        SttError::ModelNotLoaded(_) => {}
        other => panic!("expected ModelNotLoaded, got {other}"),
    }
}

// --- transcribe guard: empty audio ---

#[test]
fn test_transcribe_errors_on_empty_audio() {
    let mut engine = SttEngine::new();
    engine.load();
    let result = engine.transcribe(b"");
    assert!(result.is_err());
    match result.unwrap_err() {
        SttError::Unavailable(_) => {}
        other => panic!("expected Unavailable for empty audio, got {other}"),
    }
}

// --- transcribe: subprocess failure propagates ---

#[test]
fn test_transcribe_subprocess_failure_propagates() {
    // Point to `false` — always exits 1; audio bytes are real (non-empty).
    let engine = SttEngine {
        model_name: "whisper-small".to_string(),
        loaded: true,
        cli_override: Some("false".to_string()),
    };
    let result = engine.transcribe(b"fake wav bytes");
    assert!(result.is_err(), "subprocess exiting 1 must produce an error");
}

// --- is_available: nonexistent CLI ---

#[test]
fn test_is_available_false_for_nonexistent_cli() {
    let engine = SttEngine {
        cli_override: Some("/nonexistent/whisper_cli".to_string()),
        ..SttEngine::default()
    };
    assert!(!engine.is_available());
}

// --- locale parsing ---

#[test]
fn test_parse_locale_from_stderr_detects_english() {
    let stderr = "Detected language: en\nsome other line";
    assert_eq!(parse_locale_from_stderr(stderr), "en");
}

#[test]
fn test_parse_locale_from_stderr_empty_when_absent() {
    assert_eq!(parse_locale_from_stderr("no language info here"), "");
}

#[test]
fn test_parse_locale_case_insensitive() {
    let stderr = "DETECTED LANGUAGE: IT";
    assert_eq!(parse_locale_from_stderr(stderr), "it");
}

// --- Transcription struct ---

#[test]
fn test_transcription_fields() {
    let t = Transcription {
        text: "hello world".to_string(),
        locale: "en".to_string(),
        confidence: 0.95,
    };
    assert_eq!(t.text, "hello world");
    assert_eq!(t.locale, "en");
    assert!((t.confidence - 0.95).abs() < f32::EPSILON);
}

// --- TranscribeResponse conversion ---

#[test]
fn test_transcribe_response_from_transcription() {
    let t = Transcription {
        text: "ciao mondo".to_string(),
        locale: "it".to_string(),
        confidence: 0.8,
    };
    let resp = TranscribeResponse::from(t);
    assert_eq!(resp.text, "ciao mondo");
    assert_eq!(resp.locale, "it");
}

// --- SttError display ---

#[test]
fn test_stt_error_display_model_not_loaded() {
    let e = SttError::ModelNotLoaded("missing".to_string());
    assert!(e.to_string().contains("not loaded"));
}

#[test]
fn test_stt_error_display_timeout() {
    let e = SttError::Timeout(Duration::from_secs(60));
    assert!(e.to_string().contains("timeout"));
}

#[test]
fn test_stt_error_display_unavailable() {
    let e = SttError::Unavailable("no cli".to_string());
    assert!(e.to_string().contains("unavailable"));
}
