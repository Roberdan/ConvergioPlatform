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
/// webrtc-vad requires 10/20/30ms frames at 16kHz (160/320/480 samples).
#[cfg(feature = "voice")]
fn speech_samples(len: usize) -> Vec<i16> {
    // High-energy alternating signal that triggers voice detection.
    (0..len).map(|i| if i % 2 == 0 { 20000 } else { -20000 }).collect()
}

/// Generate near-silence samples that stay below VAD threshold.
#[cfg(feature = "voice")]
fn silence_samples(len: usize) -> Vec<i16> {
    vec![0; len]
}

/// webrtc-vad valid frame: 20ms at 16kHz = 320 samples.
#[cfg(feature = "voice")]
const FRAME_LEN: usize = 320;

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
    // check_text alone doesn't set detected (only process_frame does).
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
    let frame = make_frame(silence_samples(FRAME_LEN), 0);
    let result = det.process_frame(&frame).unwrap();
    assert!(!result);
    assert!(!det.is_detected());
}

#[cfg(feature = "voice")]
#[test]
fn process_frame_speech_then_silence() {
    let config = VoiceConfig::default();
    let mut det = WakeWordDetector::new(
        &config.wake_word,
        config.vad_threshold,
        &config.whisper_model,
    );
    // Feed multiple speech frames (>100ms min_speech) then silence frames (>300ms).
    // 20ms per frame at 320 samples.
    for i in 0..10 {
        let frame = make_frame(speech_samples(FRAME_LEN), i * 20);
        let _ = det.process_frame(&frame);
    }
    // Silence frames to end the speech segment (>300ms = 15+ frames).
    for i in 10..30 {
        let frame = make_frame(silence_samples(FRAME_LEN), i * 20);
        let _ = det.process_frame(&frame);
    }
    // With no real whisper model, transcription returns ModelNotAvailable
    // or stub text that won't contain wake word — so detected stays false.
    assert!(!det.is_detected());
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
    // Two rounds of speech + silence.
    for round in 0..2u64 {
        let base = round * 1000;
        for i in 0..10 {
            let frame = make_frame(speech_samples(FRAME_LEN), base + i * 20);
            let _ = det.process_frame(&frame);
        }
        for i in 10..30 {
            let frame = make_frame(silence_samples(FRAME_LEN), base + i * 20);
            let _ = det.process_frame(&frame);
        }
    }
    // No crash, no lingering state.
    assert!(!det.is_detected());
}
