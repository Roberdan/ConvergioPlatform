// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for kernel/tts.rs — TtsEngine, TtsError, backend detection.
//
// Included by tts.rs via:
//   #[cfg(test)] #[path = "tts_tests.rs"] mod tests;
// so `super::` refers to the tts module.

use super::*;

#[test]
fn test_engine_init() {
    let e = TtsEngine::new();
    assert!(e.loaded);
    assert!(!e.model_name.is_empty());
}

#[test]
fn test_speak_cache_hit() {
    let mut engine = TtsEngine::new();
    engine.phrase_cache.insert("it-IT:Ciao".to_string(), b"RIFF stub".to_vec());
    let first = engine.speak("Ciao", "it-IT").expect("cache hit");
    let second = engine.speak("Ciao", "it-IT").expect("second cache hit");
    assert_eq!(first, second);
}

#[test]
fn test_locale_differentiates_cache() {
    let mut engine = TtsEngine::new();
    engine.phrase_cache.insert("it-IT:Hello".to_string(), b"it".to_vec());
    engine.phrase_cache.insert("en-US:Hello".to_string(), b"en".to_vec());
    assert_ne!(
        engine.phrase_cache.get("it-IT:Hello"),
        engine.phrase_cache.get("en-US:Hello")
    );
}

#[test]
fn test_backend_detection_no_panic() {
    let _ = TtsEngine::voxtral_available();
    let _ = TtsEngine::say_available();
}

#[test]
fn test_error_display() {
    assert!(TtsError::SubprocessFailed("oops".to_string()).to_string().contains("oops"));
    assert!(TtsError::Unavailable("none".to_string()).to_string().contains("none"));
    assert!(TtsError::Template("bad".to_string()).to_string().contains("bad"));
}
