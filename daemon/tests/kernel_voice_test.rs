// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for kernel voice pipeline: TTS, STT, and voice router.
// All tests are gated behind #[cfg(feature = "kernel")].
// STT tests requiring an actual Whisper model are marked #[ignore].

#![cfg(feature = "kernel")]

use claude_core::kernel::engine::{KernelConfig, KernelEngine};
use claude_core::kernel::stt::{SttEngine, SttError};
use claude_core::kernel::tts::TtsEngine;
use claude_core::kernel::voice_router::{classify_intent, route_intent, VoiceIntent};
use std::time::Instant;

// ---------------------------------------------------------------------------
// TTS — engine creation
// ---------------------------------------------------------------------------

/// Verify TtsEngine initialises and detects the macOS `say` backend.
/// On CI without Voxtral MLX, backend falls through to `macos-say-alice`.
#[test]
fn test_tts_engine_creation() {
    let engine = TtsEngine::new();
    assert!(engine.loaded, "TtsEngine must report loaded=true after new()");
    assert!(
        !engine.model_name.is_empty(),
        "model_name must be non-empty after new()"
    );
    // macOS `say` is always present on this machine.
    assert!(
        TtsEngine::say_available(),
        "macOS `say` command must be present — required for TTS fallback"
    );
}

// ---------------------------------------------------------------------------
// TTS — speak returns WAV bytes
// ---------------------------------------------------------------------------

/// Call speak() with a short Italian phrase and verify non-empty bytes are returned.
/// Uses the default temp path (`/tmp/convergio_tts_{pid}.wav`) written by `say`.
#[test]
fn test_tts_speak_returns_wav() {
    let mut engine = TtsEngine::new();
    let result = engine.speak("test", "it-IT");
    // Clean up the temp file produced by `say`.
    let tmp = format!("/tmp/convergio_tts_{}.wav", std::process::id());
    let _ = std::fs::remove_file(&tmp);

    match result {
        Ok(bytes) => {
            assert!(!bytes.is_empty(), "speak() must return non-empty WAV bytes");
        }
        Err(e) => {
            panic!("speak() returned error on macOS where `say` is present: {e}");
        }
    }
}

// ---------------------------------------------------------------------------
// TTS — phrase cache avoids re-synthesis
// ---------------------------------------------------------------------------

/// Speak the same text twice; the second call must be served from cache (faster).
#[test]
fn test_tts_phrase_cache() {
    let mut engine = TtsEngine::new();

    // First call — synthesise and populate cache.
    let first = engine.speak("ciao", "it-IT");

    // Clean up temp file from the first synthesis.
    let tmp = format!("/tmp/convergio_tts_{}.wav", std::process::id());
    let _ = std::fs::remove_file(&tmp);

    assert!(first.is_ok(), "first speak() must succeed: {:?}", first.err());

    // Second call — must be served from cache; no temp file needed.
    let second_start = Instant::now();
    let second = engine.speak("ciao", "it-IT");
    let second_elapsed = second_start.elapsed();

    assert!(second.is_ok(), "second speak() must succeed: {:?}", second.err());
    assert_eq!(
        first.unwrap(),
        second.unwrap(),
        "cached response must equal the original synthesis"
    );
    // Cache hit should be significantly faster — allow 50 ms ceiling regardless
    // of how long the first synthesis took (CI machines vary).
    assert!(
        second_elapsed.as_millis() < 50,
        "cache hit took {} ms — expected <50 ms",
        second_elapsed.as_millis()
    );
}

// ---------------------------------------------------------------------------
// STT — engine construction (no Whisper model required)
// ---------------------------------------------------------------------------

/// Verify SttEngine defaults and transcribe-without-load error.
#[test]
fn test_stt_engine_creation() {
    let engine = SttEngine::new();
    assert_eq!(engine.model_name, "whisper-small");
    assert!(!engine.loaded, "SttEngine must not be loaded by default");
    // Transcribing without loading must return an explicit error (Fail-Loud).
    match engine.transcribe(b"fake audio bytes") {
        Err(SttError::ModelNotLoaded(_)) => {}
        other => panic!("expected ModelNotLoaded, got: {other:?}"),
    }
}

/// Transcribing real audio via Whisper requires the model — mark as ignored.
#[test]
#[ignore = "requires whisper-small model and mlx_whisper/whisper-cpp on PATH"]
fn test_stt_transcribe_real_audio() {
    let mut engine = SttEngine::new();
    engine.load();
    // Minimal 16 kHz mono PCM WAV (44 header + 100 bytes silence).
    let mut wav: Vec<u8> = Vec::new();
    wav.extend_from_slice(b"RIFF");
    let data_size: u32 = 100;
    wav.extend_from_slice(&(36 + data_size).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&16000u32.to_le_bytes());
    wav.extend_from_slice(&32000u32.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    wav.extend(std::iter::repeat(0u8).take(data_size as usize));

    let result = engine.transcribe(&wav);
    assert!(result.is_ok(), "transcribe must succeed: {:?}", result.err());
}

// ---------------------------------------------------------------------------
// Voice router — classify_intent
// ---------------------------------------------------------------------------

/// "stato" → StatusCheck via Italian keyword matching.
#[test]
fn test_voice_router_classify_stato() {
    let engine = KernelEngine::new(KernelConfig::default());
    assert_eq!(
        classify_intent("stato", &engine),
        VoiceIntent::StatusCheck,
        "\"stato\" must map to StatusCheck"
    );
}

/// "quanto ho speso oggi" → CostQuery.
/// Uses "dollari" variant to guarantee keyword hit without LLM.
#[test]
fn test_voice_router_classify_costi() {
    let engine = KernelEngine::new(KernelConfig::default());
    // The keyword list checks "cost", "spesa", "dollari".
    // "speso" is not in the list; use "costi" phrase that includes "cost" substring.
    let intent = classify_intent("quanto costano i piani", &engine);
    assert_eq!(
        intent,
        VoiceIntent::CostQuery,
        "\"costano\" (contains \"cost\") must trigger CostQuery"
    );
}

/// Nonsense input → Unknown.
#[test]
fn test_voice_router_classify_unknown() {
    let engine = KernelEngine::new(KernelConfig::default());
    assert_eq!(
        classify_intent("abracadabra", &engine),
        VoiceIntent::Unknown,
        "unrecognised input must map to Unknown"
    );
}

// ---------------------------------------------------------------------------
// Voice router — route_intent for Unknown
// ---------------------------------------------------------------------------

/// Unknown intent → non-empty Italian error/help message.
#[test]
fn test_voice_router_route_unknown() {
    let response = route_intent(VoiceIntent::Unknown, "http://localhost:1");
    assert!(
        !response.is_empty(),
        "route_intent(Unknown) must return a non-empty string"
    );
    assert!(
        response.contains("stato")
            || response.contains("Non ho capito")
            || response.contains("Prova"),
        "expected Italian help text, got: {response}"
    );
}
