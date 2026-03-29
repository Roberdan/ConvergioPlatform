use super::wake_word::WakeWordDetector;

#[cfg(feature = "voice")]
use super::types::{AudioFrame, VoiceConfig};

#[cfg(feature = "voice")]
fn make_frame(samples: Vec<i16>, timestamp_ms: u64) -> AudioFrame {
    AudioFrame {
        samples,
        sample_rate: 16000,
        timestamp_ms,
    }
}

/// Generate a loud tone that exceeds VAD threshold, simulating speech.
#[cfg(feature = "voice")]
fn speech_samples(len: usize) -> Vec<i16> {
    (0..len).map(|i| ((i % 50) as i16 * 500) - 12500).collect()
}

/// Generate near-silence samples that stay below VAD threshold.
#[cfg(feature = "voice")]
fn silence_samples(len: usize) -> Vec<i16> {
    vec![10; len]
}

#[test]
fn new_detector_starts_inactive() {
    let det = WakeWordDetector::new("convergio", 0.5, "small");
    assert!(!det.is_detected());
}

#[test]
fn reset_clears_detection_state() {
    let mut det = WakeWordDetector::new("convergio", 0.5, "small");
    det.reset();
    assert!(!det.is_detected());
}

#[test]
fn wake_word_accessor_returns_configured_word() {
    let det = WakeWordDetector::new("jarvis", 0.5, "small");
    assert_eq!(det.wake_word(), "jarvis");
}

#[test]
fn check_text_detects_wake_word_in_transcription() {
    let mut det = WakeWordDetector::new("convergio", 0.5, "small");
    assert!(det.check_text("hey convergio how are you").unwrap());
}

#[test]
fn check_text_case_insensitive() {
    let mut det = WakeWordDetector::new("convergio", 0.5, "small");
    assert!(det.check_text("Hey CONVERGIO").unwrap());
}

#[test]
fn check_text_no_false_positive() {
    let mut det = WakeWordDetector::new("convergio", 0.5, "small");
    assert!(!det.check_text("the weather is nice today").unwrap());
}

#[test]
fn check_text_empty_string() {
    let mut det = WakeWordDetector::new("convergio", 0.5, "small");
    assert!(!det.check_text("").unwrap());
}

#[test]
fn detection_state_set_by_check_text() {
    let mut det = WakeWordDetector::new("jarvis", 0.5, "small");
    assert!(!det.is_detected());
    // check_text alone doesn't set detected (that's only process_frame).
    det.check_text("hey jarvis").unwrap();
    assert!(!det.is_detected());
}

// --- VAD + Whisper micro-transcription tests (voice feature only) ---

#[cfg(feature = "voice")]
#[test]
fn silence_does_not_trigger() {
    let config = VoiceConfig::default();
    let mut det = WakeWordDetector::new(
        &config.wake_word,
        config.vad_threshold,
        &config.whisper_model,
    );
    let frame = make_frame(silence_samples(1600), 0);
    let result = det.process_frame(&frame).unwrap();
    assert!(!result);
    assert!(!det.is_detected());
}

#[cfg(feature = "voice")]
#[test]
fn process_frame_returns_result() {
    let config = VoiceConfig::default();
    let mut det = WakeWordDetector::new(
        &config.wake_word,
        config.vad_threshold,
        &config.whisper_model,
    );
    // Speech followed by silence should produce a segment.
    let speech = make_frame(speech_samples(1600), 0);
    let silence = make_frame(silence_samples(1600), 500);
    let r1 = det.process_frame(&speech).unwrap();
    let r2 = det.process_frame(&silence).unwrap();
    // With stub whisper, transcription won't contain the wake word.
    assert!(!r1);
    assert!(!r2);
}

#[cfg(feature = "voice")]
#[test]
fn multiple_segments_independent() {
    let config = VoiceConfig::default();
    let mut det = WakeWordDetector::new(
        &config.wake_word,
        config.vad_threshold,
        &config.whisper_model,
    );
    for ts_base in [0u64, 1000] {
        let speech = make_frame(speech_samples(1600), ts_base);
        let silence = make_frame(silence_samples(1600), ts_base + 500);
        let _ = det.process_frame(&speech);
        let _ = det.process_frame(&silence);
    }
    assert!(!det.is_detected());
}
