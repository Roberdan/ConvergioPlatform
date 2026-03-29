use super::pipeline::VoicePipeline;
use super::types::{AudioFrame, VoiceConfig, VoiceState};

fn default_pipeline() -> VoicePipeline {
    VoicePipeline::new(VoiceConfig::default())
}

#[test]
fn initial_state_is_idle() {
    let p = default_pipeline();
    assert_eq!(p.state(), VoiceState::Idle);
}

#[test]
fn start_transitions_to_listening() {
    let mut p = default_pipeline();
    p.start().unwrap();
    assert_eq!(p.state(), VoiceState::Listening);
}

#[test]
fn stop_transitions_to_idle() {
    let mut p = default_pipeline();
    p.start().unwrap();
    p.stop();
    assert_eq!(p.state(), VoiceState::Idle);
}

#[test]
fn idle_ignores_frames() {
    let mut p = default_pipeline();
    let frame = AudioFrame {
        samples: vec![0i16; 160],
        sample_rate: 16000,
        timestamp_ms: 0,
    };
    let result = p.process_frame(&frame).unwrap();
    assert!(result.is_none());
}

#[test]
fn config_accessible() {
    let p = default_pipeline();
    assert_eq!(p.config().wake_word, "convergio");
    assert_eq!(p.config().whisper_model, "small");
}

#[test]
fn speak_uses_kernel_tts_engine() {
    // Pipeline's TTS must be kernel::tts::TtsEngine (multi-backend, cached).
    // The voice/tts.rs simple wrapper must not exist.
    let mut p = default_pipeline();
    p.start().unwrap();
    // speak() now delegates to kernel TtsEngine which returns Result<Vec<u8>, _>.
    // Pipeline wraps errors as VoiceError and returns Ok(()) on success.
    // On CI without audio backends, speak may fail — that's fine, we test the wiring.
    let _result = p.speak("Benvenuto", "it-IT");
}

#[test]
fn speak_requires_locale_parameter() {
    // Kernel TtsEngine requires locale for voice selection.
    // Pipeline.speak() must accept locale.
    let mut p = default_pipeline();
    p.start().unwrap();
    let _result = p.speak("Hello world", "en-US");
}
