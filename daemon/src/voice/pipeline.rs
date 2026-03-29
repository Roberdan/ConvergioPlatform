use super::intent::{extract_intent, Intent};
use super::types::{AudioFrame, VoiceConfig, VoiceError, VoiceState};
use super::vad::VoiceActivityDetector;
use super::wake_word::WakeWordDetector;
use super::whisper::WhisperEngine;
use crate::kernel::tts::TtsEngine;

/// Full voice pipeline: VAD → Wake Word → ASR → Intent → TTS.
/// State machine: Idle → Listening → Processing → Speaking → Idle.
pub struct VoicePipeline {
    state: VoiceState,
    config: VoiceConfig,
    vad: VoiceActivityDetector,
    wake_detector: WakeWordDetector,
    whisper: WhisperEngine,
    tts: TtsEngine,
    wake_detected: bool,
}

impl VoicePipeline {
    pub fn new(config: VoiceConfig) -> Self {
        let vad = VoiceActivityDetector::new(config.vad_threshold);
        let wake = WakeWordDetector::new(
            &config.wake_word,
            config.vad_threshold,
            &config.whisper_model,
        );
        let whisper = WhisperEngine::new(&config.whisper_model, config.prefer_local);
        let tts = TtsEngine::new();
        Self {
            state: VoiceState::Idle,
            config,
            vad,
            wake_detector: wake,
            whisper,
            tts,
            wake_detected: false,
        }
    }

    pub fn state(&self) -> VoiceState {
        self.state
    }

    pub fn config(&self) -> &VoiceConfig {
        &self.config
    }

    /// Start listening for audio.
    pub fn start(&mut self) -> Result<(), VoiceError> {
        self.state = VoiceState::Listening;
        self.vad.reset();
        self.wake_detector.reset();
        self.wake_detected = false;
        Ok(())
    }

    /// Stop the pipeline.
    pub fn stop(&mut self) {
        self.state = VoiceState::Idle;
        self.vad.reset();
        self.wake_detected = false;
    }

    /// Process an incoming audio frame through the pipeline.
    /// Returns an Intent if the full pipeline completes.
    pub fn process_frame(&mut self, frame: &AudioFrame) -> Result<Option<Intent>, VoiceError> {
        if self.state == VoiceState::Idle {
            return Ok(None);
        }

        // Step 1: VAD — detect speech segments.
        let segment = self.vad.process(frame)?;
        let Some(segment) = segment else {
            return Ok(None);
        };

        // Step 2: ASR — transcribe the speech segment.
        self.state = VoiceState::Processing;
        let transcription = self.whisper.transcribe(&segment)?;

        // Step 3: Wake word check (if not yet activated).
        if !self.wake_detected {
            if self.wake_detector.check_text(&transcription.text)? {
                self.wake_detected = true;
            }
            self.state = VoiceState::Listening;
            return Ok(None);
        }

        // Step 4: Intent extraction.
        let intent = extract_intent(&transcription.text)?;

        // Step 5: Reset for next utterance.
        self.wake_detected = false;
        self.state = VoiceState::Listening;

        Ok(Some(intent))
    }

    /// Speak a response via TTS (delegates to kernel TtsEngine).
    /// Returns synthesised WAV bytes on success.
    pub fn speak(&mut self, text: &str, locale: &str) -> Result<Vec<u8>, VoiceError> {
        let prev = self.state;
        self.state = VoiceState::Speaking;
        let result = self.tts.speak(text, locale).map_err(|e| {
            VoiceError::TtsError(e.to_string())
        });
        self.state = prev;
        result
    }
}
