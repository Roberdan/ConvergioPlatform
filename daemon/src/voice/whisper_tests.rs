use super::types::{SpeechSegment, VoiceError};
use super::whisper::{Transcription, WhisperEngine};

/// Transcription struct stores expected fields.
#[test]
fn transcription_fields() {
    let t = Transcription {
        text: "hello world".to_string(),
        language: "en".to_string(),
        confidence: 0.95,
        is_partial: false,
    };
    assert_eq!(t.text, "hello world");
    assert_eq!(t.language, "en");
    assert!((t.confidence - 0.95).abs() < f32::EPSILON);
    assert!(!t.is_partial);
}

/// Clone and Debug are derived on Transcription.
#[test]
fn transcription_clone_debug() {
    let t = Transcription {
        text: "test".to_string(),
        language: "it".to_string(),
        confidence: 0.8,
        is_partial: true,
    };
    let cloned = t.clone();
    assert_eq!(cloned.text, t.text);
    assert!(format!("{:?}", t).contains("test"));
}

/// Engine construction sets model size and mode.
#[test]
fn engine_new_stores_config() {
    let engine = WhisperEngine::new("small", true);
    assert_eq!(engine.model_size(), "small");
    assert!(engine.prefers_local());
}

/// Engine construction with medium model and API mode.
#[test]
fn engine_new_medium_api() {
    let engine = WhisperEngine::new("medium", false);
    assert_eq!(engine.model_size(), "medium");
    assert!(!engine.prefers_local());
}

/// Empty segment returns AsrError.
#[test]
fn transcribe_empty_segment_returns_error() {
    let engine = WhisperEngine::new("small", true);
    let segment = SpeechSegment {
        start_ms: 0,
        end_ms: 100,
        samples: vec![],
    };
    let result = engine.transcribe(&segment);
    assert!(result.is_err());
    match result.unwrap_err() {
        VoiceError::AsrError(msg) => assert!(msg.contains("empty"), "got: {msg}"),
        other => panic!("expected AsrError, got: {other:?}"),
    }
}

/// Transcribe returns ModelNotAvailable when model file is missing.
/// This is the expected runtime behavior since tests don't have model binaries.
#[test]
fn transcribe_without_model_returns_model_error() {
    let engine = WhisperEngine::new("small", true);
    let segment = SpeechSegment {
        start_ms: 0,
        end_ms: 1000,
        samples: vec![0i16; 16000], // 1 second of silence at 16kHz
    };
    let result = engine.transcribe(&segment);
    // Without a model file loaded, expect ModelNotAvailable
    assert!(result.is_err());
    match result.unwrap_err() {
        VoiceError::ModelNotAvailable(msg) => {
            assert!(msg.contains("model"), "got: {msg}");
        }
        VoiceError::AsrError(_) => {
            // Also acceptable — depends on whisper-rs error path
        }
        other => panic!("expected ModelNotAvailable or AsrError, got: {other:?}"),
    }
}

/// Samples-to-f32 conversion produces normalized values in [-1, 1].
#[test]
fn samples_to_f32_normalization() {
    use super::whisper::samples_to_f32;

    let samples = vec![0i16, i16::MAX, i16::MIN, 16384];
    let floats = samples_to_f32(&samples);

    assert_eq!(floats.len(), 4);
    assert!((floats[0]).abs() < f32::EPSILON); // 0 → 0.0
    assert!((floats[1] - 1.0).abs() < 0.001); // MAX → ~1.0
    assert!((floats[2] + 1.0).abs() < 0.001); // MIN → ~-1.0
    assert!(floats[3] > 0.0 && floats[3] < 1.0); // mid-range positive
}

/// Samples-to-f32 with empty input returns empty.
#[test]
fn samples_to_f32_empty() {
    use super::whisper::samples_to_f32;
    let floats = samples_to_f32(&[]);
    assert!(floats.is_empty());
}

/// model_path returns expected path pattern.
#[test]
fn model_path_pattern() {
    let engine = WhisperEngine::new("small", true);
    let path = engine.model_path();
    assert!(path.contains("whisper"), "path should contain 'whisper': {path}");
    assert!(path.contains("small"), "path should contain model size: {path}");
}
