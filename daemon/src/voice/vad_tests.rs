use super::types::AudioFrame;
use super::vad::VoiceActivityDetector;

fn silence_frame(ts: u64) -> AudioFrame {
    AudioFrame { samples: vec![0i16; 160], sample_rate: 16000, timestamp_ms: ts }
}

fn speech_frame(ts: u64) -> AudioFrame {
    // Loud signal — high energy
    AudioFrame { samples: vec![20000i16; 160], sample_rate: 16000, timestamp_ms: ts }
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
    // Speech frames
    for ms in (0..200).step_by(10) {
        vad.process(&speech_frame(ms)).unwrap();
    }
    assert!(vad.is_in_speech());
    // Silence frames to end segment
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
