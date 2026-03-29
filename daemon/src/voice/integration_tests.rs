//! Integration tests: cross-module interactions across the voice engine.
//! VAD->Whisper, audio_util->VAD, wake word->intent, config propagation,
//! serde roundtrips, error boundaries, and end-to-end data paths.

use super::audio_util::{resample, stereo_to_mono};
use super::intent::{extract_intent, IntentType};
use super::types::{AudioFrame, SpeechSegment, VoiceConfig, VoiceError, VoiceState};
use super::vad::VoiceActivityDetector;
use super::wake_word::WakeWordDetector;
use super::whisper::{samples_to_f32, WhisperEngine};

fn silence_frame(ts: u64) -> AudioFrame {
    AudioFrame { samples: vec![0i16; 160], sample_rate: 16000, timestamp_ms: ts }
}

fn speech_frame(ts: u64) -> AudioFrame {
    let mut seed: u64 = 12345 + ts * 67890;
    let samples: Vec<i16> = (0..160)
        .map(|_| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 33) as i32 - 1_000_000) as i16
        })
        .collect();
    AudioFrame { samples, sample_rate: 16000, timestamp_ms: ts }
}

fn produce_speech_segment(vad: &mut VoiceActivityDetector) -> SpeechSegment {
    for ms in (0..200).step_by(10) {
        vad.process(&speech_frame(ms)).unwrap();
    }
    for ms in (200..1200).step_by(10) {
        if let Some(seg) = vad.process(&silence_frame(ms)).unwrap() {
            return seg;
        }
    }
    panic!("VAD did not emit a segment after speech + silence");
}

// -- VAD -> Whisper -----------------------------------------------------------

#[test]
fn vad_segment_fed_to_whisper_engine() {
    let mut vad = VoiceActivityDetector::new(0.1);
    let segment = produce_speech_segment(&mut vad);
    assert!(!segment.samples.is_empty());
    assert!(segment.start_ms < segment.end_ms);
    let engine = WhisperEngine::new("small", true);
    let result = engine.transcribe(&segment);
    assert!(result.is_err());
    match result.unwrap_err() {
        VoiceError::ModelNotAvailable(_) | VoiceError::AsrError(_) => {}
        other => panic!("expected model/asr error, got: {other:?}"),
    }
}

#[test]
fn vad_segment_samples_convertible_to_f32() {
    let mut vad = VoiceActivityDetector::new(0.1);
    let segment = produce_speech_segment(&mut vad);
    let f32_samples = samples_to_f32(&segment.samples);
    assert_eq!(f32_samples.len(), segment.samples.len());
    for &v in &f32_samples {
        assert!((-1.0..=1.0).contains(&v), "sample out of range: {v}");
    }
}

// -- audio_util -> VAD --------------------------------------------------------

#[test]
fn resampled_48k_stereo_produces_valid_vad_frames() {
    let stereo_48k: Vec<i16> = (0..960).map(|i| (i % 200) as i16 * 100).collect();
    let mono = stereo_to_mono(&stereo_48k);
    assert_eq!(mono.len(), 480);
    let resampled = resample(&mono, 48000, 16000);
    assert_eq!(resampled.len(), 160);
    let frame = AudioFrame { samples: resampled, sample_rate: 16000, timestamp_ms: 0 };
    let mut vad = VoiceActivityDetector::new(0.5);
    assert!(vad.process(&frame).is_ok());
}

#[test]
fn resampled_44100_mono_produces_valid_vad_frames() {
    let mono_44k: Vec<i16> = vec![500; 441];
    let resampled = resample(&mono_44k, 44100, 16000);
    assert!(resampled.len() >= 159 && resampled.len() <= 161);
    let mut samples = resampled;
    samples.resize(160, 0);
    let frame = AudioFrame { samples, sample_rate: 16000, timestamp_ms: 0 };
    let mut vad = VoiceActivityDetector::new(0.3);
    assert!(vad.process(&frame).is_ok());
}

// -- Wake word -> intent ------------------------------------------------------

#[test]
fn wake_word_detected_then_intent_extracted() {
    let mut det = WakeWordDetector::new("convergio", 0.5, "small");
    assert!(det.check_text("hey convergio show me the plans").unwrap());
    let intent = extract_intent("list all plans").unwrap();
    assert_eq!(intent.intent_type, IntentType::Command);
    assert!(intent.command.as_ref().unwrap().contains("plan"));
}

#[test]
fn no_wake_word_means_no_intent_processing() {
    let mut det = WakeWordDetector::new("convergio", 0.5, "small");
    assert!(!det.check_text("the weather is nice today").unwrap());
    let intent = extract_intent("the weather is nice today").unwrap();
    assert_eq!(intent.intent_type, IntentType::Ambiguous);
}

#[test]
fn custom_wake_word_with_intent_chain() {
    let mut det = WakeWordDetector::new("jarvis", 0.5, "small");
    assert!(det.check_text("jarvis stop listening").unwrap());
    let intent = extract_intent("stop listening").unwrap();
    assert_eq!(intent.intent_type, IntentType::Control);
    assert!(intent.command.as_ref().unwrap().contains("stop"));
}

// -- VoiceConfig propagation --------------------------------------------------

#[test]
fn config_propagates_to_all_components() {
    let config = VoiceConfig {
        vad_threshold: 0.8, wake_word: "jarvis".to_string(),
        whisper_model: "medium".to_string(), tts_voice: "Luca".to_string(),
        tts_rate: 1.5, prefer_local: false,
    };
    let vad = VoiceActivityDetector::new(config.vad_threshold);
    assert!(!vad.is_in_speech());
    let det = WakeWordDetector::new(&config.wake_word, config.vad_threshold, &config.whisper_model);
    assert_eq!(det.wake_word(), "jarvis");
    let engine = WhisperEngine::new(&config.whisper_model, config.prefer_local);
    assert_eq!(engine.model_size(), "medium");
    assert!(!engine.prefers_local());
}

// -- VoiceState + VoiceConfig serde -------------------------------------------

#[test]
fn voice_state_display_and_serde_roundtrip() {
    let states = [
        VoiceState::Idle, VoiceState::Listening, VoiceState::WakeDetected,
        VoiceState::Processing, VoiceState::Speaking,
    ];
    for state in &states {
        assert!(!format!("{state}").is_empty());
        let json = serde_json::to_string(state).unwrap();
        let back: VoiceState = serde_json::from_str(&json).unwrap();
        assert_eq!(*state, back);
    }
}

#[test]
fn voice_config_serde_roundtrip() {
    let config = VoiceConfig {
        vad_threshold: 0.7, wake_word: "jarvis".to_string(),
        whisper_model: "medium".to_string(), tts_voice: "Federica".to_string(),
        tts_rate: 0.9, prefer_local: true,
    };
    let json = serde_json::to_string(&config).unwrap();
    let back: VoiceConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.wake_word, "jarvis");
    assert_eq!(back.whisper_model, "medium");
    assert!((back.vad_threshold - 0.7).abs() < f32::EPSILON);
}

// -- VoiceError across module boundaries --------------------------------------

#[test]
fn voice_error_variants_display_correctly() {
    let errors = vec![
        VoiceError::AudioError("no device".into()),
        VoiceError::VadError("bad frame".into()),
        VoiceError::AsrError("decode fail".into()),
        VoiceError::TtsError("no voice".into()),
        VoiceError::IntentError("parse fail".into()),
        VoiceError::PipelineError("stalled".into()),
        VoiceError::ModelNotAvailable("ggml-small.bin".into()),
    ];
    for err in &errors {
        assert!(!err.to_string().is_empty());
    }
}

// -- Multi-stage VAD ----------------------------------------------------------

#[test]
fn vad_produces_multiple_independent_segments() {
    let mut vad = VoiceActivityDetector::new(0.1);
    let seg1 = produce_speech_segment(&mut vad);
    assert!(!seg1.samples.is_empty());
    vad.reset();
    let seg2_start = 2000u64;
    for ms in (seg2_start..seg2_start + 200).step_by(10) {
        vad.process(&speech_frame(ms)).unwrap();
    }
    let mut seg2 = None;
    for ms in (seg2_start + 200..seg2_start + 1200).step_by(10) {
        if let Some(s) = vad.process(&silence_frame(ms)).unwrap() {
            seg2 = Some(s);
            break;
        }
    }
    let seg2 = seg2.expect("second segment must emit");
    assert!(seg2.start_ms >= seg2_start);
    assert_ne!(seg1.start_ms, seg2.start_ms);
}

// -- End-to-end: audio_util -> VAD -> Whisper (no model) ----------------------

#[test]
fn full_audio_to_transcription_chain_without_model() {
    let mut vad = VoiceActivityDetector::new(0.1);
    for frame_idx in 0..20u64 {
        let ts = frame_idx * 10;
        let raw = speech_frame(ts);
        let stereo: Vec<i16> = raw.samples.iter().flat_map(|&s| [s, s]).collect();
        let mono = stereo_to_mono(&stereo);
        assert_eq!(mono.len(), raw.samples.len());
        let frame = AudioFrame { samples: mono, sample_rate: 16000, timestamp_ms: ts };
        vad.process(&frame).unwrap();
    }
    let mut segment = None;
    for ms in (200..1200).step_by(10) {
        if let Some(s) = vad.process(&silence_frame(ms)).unwrap() {
            segment = Some(s);
            break;
        }
    }
    let segment = segment.expect("segment must emit");
    let engine = WhisperEngine::new("small", true);
    assert!(engine.transcribe(&segment).is_err());
}

// -- SpeechSegment field integrity --------------------------------------------

#[test]
fn speech_segment_timestamps_monotonic() {
    let mut vad = VoiceActivityDetector::new(0.1);
    let seg = produce_speech_segment(&mut vad);
    assert!(seg.start_ms < seg.end_ms);
    assert!(seg.end_ms - seg.start_ms >= 100);
    let expected = ((seg.end_ms - seg.start_ms) * 16) as usize;
    assert!(seg.samples.len() > expected / 2,
        "too few samples: {} vs expected ~{expected}", seg.samples.len());
}
