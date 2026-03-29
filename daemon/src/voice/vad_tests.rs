use super::types::AudioFrame;
use super::vad::{threshold_to_vad_mode, VadAggressiveness, VoiceActivityDetector};

fn silence_frame(ts: u64) -> AudioFrame {
    AudioFrame { samples: vec![0i16; 160], sample_rate: 16000, timestamp_ms: ts }
}

fn speech_frame(ts: u64) -> AudioFrame {
    // Pseudo-random broadband noise at speech amplitude — webrtc-vad detects this as voice.
    // Uses a simple LCG seeded by timestamp to produce varying waveforms per frame.
    let mut seed: u64 = 12345 + ts * 67890;
    let mut samples = vec![0i16; 160];
    for s in samples.iter_mut() {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        *s = ((seed >> 33) as i32 - 1_000_000) as i16;
    }
    AudioFrame { samples, sample_rate: 16000, timestamp_ms: ts }
}

#[test]
fn silence_produces_no_segment() {
    let mut vad = VoiceActivityDetector::new(0.3);
    let result = vad.process(&silence_frame(0)).unwrap();
    assert!(result.is_none());
}

#[test]
fn speech_then_silence_produces_segment() {
    let mut vad = VoiceActivityDetector::new(0.1);
    // Speech frames (200ms of loud audio).
    let mut voice_count = 0;
    for ms in (0..200).step_by(10) {
        let frame = speech_frame(ms);
        // Verify webrtc-vad sees these as voice before processing.
        let mut probe = webrtc_vad::Vad::new_with_rate(webrtc_vad::SampleRate::Rate16kHz);
        probe.set_mode(webrtc_vad::VadMode::Quality);
        if probe.is_voice_segment(&frame.samples).unwrap_or(false) {
            voice_count += 1;
        }
        vad.process(&frame).unwrap();
    }
    eprintln!("voice_count={voice_count}/20, in_speech={}", vad.is_in_speech());
    assert!(vad.is_in_speech(), "VAD must detect speech (voice_count={voice_count}/20)");
    // Silence frames to end segment.
    let mut segment = None;
    for ms in (200..600).step_by(10) {
        if let Some(s) = vad.process(&silence_frame(ms)).unwrap() {
            segment = Some(s);
            break;
        }
    }
    assert!(segment.is_some());
    let seg = segment.unwrap();
    assert!(seg.start_ms < seg.end_ms);
    assert!(!seg.samples.is_empty());
}

#[test]
fn reset_clears_state() {
    let mut vad = VoiceActivityDetector::new(0.1);
    vad.process(&speech_frame(0)).unwrap();
    assert!(vad.is_in_speech());
    vad.reset();
    assert!(!vad.is_in_speech());
}

#[test]
fn threshold_maps_to_vad_mode() {
    // Low threshold = Quality (most permissive).
    assert_eq!(threshold_to_vad_mode(0.1), VadAggressiveness::Quality);
    assert_eq!(threshold_to_vad_mode(0.3), VadAggressiveness::LowBitrate);
    assert_eq!(threshold_to_vad_mode(0.6), VadAggressiveness::Aggressive);
    assert_eq!(threshold_to_vad_mode(0.9), VadAggressiveness::VeryAggressive);
}

#[test]
fn threshold_clamped_to_valid_range() {
    // Out-of-range values should not panic.
    let _vad_low = VoiceActivityDetector::new(-1.0);
    let _vad_high = VoiceActivityDetector::new(5.0);
}

#[test]
fn short_speech_below_minimum_not_emitted() {
    let mut vad = VoiceActivityDetector::new(0.1);
    // Single speech frame (10ms) — below min_speech_ms (100ms).
    vad.process(&speech_frame(0)).unwrap();
    // Immediate silence — speech too short, no segment.
    for ms in (10..400).step_by(10) {
        let result = vad.process(&silence_frame(ms)).unwrap();
        assert!(result.is_none(), "short speech should not produce segment at {ms}ms");
    }
}

#[test]
fn frame_size_must_be_valid_for_webrtc_vad() {
    let mut vad = VoiceActivityDetector::new(0.3);
    // 160 samples = 10ms at 16kHz — valid for webrtc-vad.
    let valid = AudioFrame { samples: vec![0i16; 160], sample_rate: 16000, timestamp_ms: 0 };
    assert!(vad.process(&valid).is_ok());

    // 320 samples = 20ms — also valid.
    let valid_20ms = AudioFrame { samples: vec![0i16; 320], sample_rate: 16000, timestamp_ms: 10 };
    assert!(vad.process(&valid_20ms).is_ok());
}

#[test]
fn invalid_frame_size_returns_error() {
    let mut vad = VoiceActivityDetector::new(0.3);
    // 100 samples — not a valid webrtc-vad frame size.
    let invalid = AudioFrame { samples: vec![0i16; 100], sample_rate: 16000, timestamp_ms: 0 };
    assert!(vad.process(&invalid).is_err());
}
