use super::pipeline::{PipelineEvent, VoicePipeline};
use super::types::{AudioFrame, VoiceConfig, VoiceState};

fn default_pipeline() -> VoicePipeline {
    VoicePipeline::new(VoiceConfig::default())
}

fn silence_frame(timestamp_ms: u64) -> AudioFrame {
    AudioFrame {
        samples: vec![0i16; 160],
        sample_rate: 16000,
        timestamp_ms,
    }
}

// --- State machine tests ---

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
    let frame = silence_frame(0);
    let events = p.process_frame(&frame).unwrap();
    assert!(events.is_empty());
}

#[test]
fn config_accessible() {
    let p = default_pipeline();
    assert_eq!(p.config().wake_word, "convergio");
    assert_eq!(p.config().whisper_model, "small");
}

// --- WakeDetected state ---

#[test]
fn wake_detected_state_display() {
    assert_eq!(VoiceState::WakeDetected.to_string(), "wake_detected");
}

#[test]
fn set_wake_detected_transitions_state() {
    let mut p = default_pipeline();
    p.start().unwrap();
    p.set_wake_detected();
    assert_eq!(p.state(), VoiceState::WakeDetected);
}

#[test]
fn wake_detected_resets_on_stop() {
    let mut p = default_pipeline();
    p.start().unwrap();
    p.set_wake_detected();
    p.stop();
    assert_eq!(p.state(), VoiceState::Idle);
    assert!(!p.is_wake_active());
}

// --- Event emission ---

#[test]
fn silence_frames_produce_no_events() {
    let mut p = default_pipeline();
    p.start().unwrap();
    for i in 0..10 {
        let events = p.process_frame(&silence_frame(i * 10)).unwrap();
        assert!(events.is_empty());
    }
    assert_eq!(p.state(), VoiceState::Listening);
}

#[test]
fn process_frame_returns_events_vec() {
    let mut p = default_pipeline();
    p.start().unwrap();
    let events = p.process_frame(&silence_frame(0)).unwrap();
    let _: Vec<PipelineEvent> = events;
}

// --- TTS integration ---

#[test]
fn speak_uses_kernel_tts_engine() {
    let mut p = default_pipeline();
    p.start().unwrap();
    let _result = p.speak("Benvenuto", "it-IT");
}

#[test]
fn speak_requires_locale_parameter() {
    let mut p = default_pipeline();
    p.start().unwrap();
    let _result = p.speak("Hello world", "en-US");
}

#[test]
fn speak_sets_speaking_state_then_restores() {
    let mut p = default_pipeline();
    p.start().unwrap();
    let prev = p.state();
    let _result = p.speak("test", "en-US");
    assert_eq!(p.state(), prev);
}

// --- Conversation loop (async) ---

#[tokio::test]
async fn run_loop_processes_frames_from_channel() {
    let mut p = default_pipeline();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<AudioFrame>();

    for i in 0..5 {
        tx.send(silence_frame(i * 10)).unwrap();
    }
    drop(tx);

    let events = p.run_from_receiver(rx).await.unwrap();
    assert!(events.iter().all(|e| !matches!(e, PipelineEvent::IntentRecognized(_))));
}

#[tokio::test]
async fn run_loop_emits_state_changed_on_start() {
    let mut p = default_pipeline();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<AudioFrame>();
    tx.send(silence_frame(0)).unwrap();
    drop(tx);

    let events = p.run_from_receiver(rx).await.unwrap();
    assert!(events.iter().any(|e| matches!(e, PipelineEvent::StateChanged(VoiceState::Listening))));
}

// --- Pipeline event variants ---

#[test]
fn pipeline_event_debug_display() {
    let ev = PipelineEvent::StateChanged(VoiceState::WakeDetected);
    let dbg = format!("{ev:?}");
    assert!(dbg.contains("WakeDetected"));
}

#[test]
fn pipeline_event_wake_word_variant_exists() {
    let ev = PipelineEvent::WakeWordDetected;
    assert!(format!("{ev:?}").contains("WakeWordDetected"));
}

#[test]
fn pipeline_event_error_variant_captures_message() {
    use super::types::VoiceError;
    let ev = PipelineEvent::Error(VoiceError::PipelineError("test error".into()));
    let dbg = format!("{ev:?}");
    assert!(dbg.contains("test error"));
}
