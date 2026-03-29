use super::types::VoiceError;

#[cfg(feature = "voice")]
use super::types::AudioFrame;
#[cfg(feature = "voice")]
use super::vad::VoiceActivityDetector;
#[cfg(feature = "voice")]
use super::whisper::WhisperEngine;

/// Wake word detector using VAD + Whisper micro-transcription.
///
/// When the `voice` feature is enabled, raw audio frames are fed in; when VAD
/// emits a speech segment the detector runs a fast Whisper micro-transcription
/// and checks the result for the wake word. This keeps wake word detection
/// lightweight — only short bursts are transcribed, not full utterances.
///
/// Without the `voice` feature, only text-based detection via `check_text` is
/// available (used by the pipeline after external transcription).
pub struct WakeWordDetector {
    wake_word: String,
    #[cfg(feature = "voice")]
    vad: VoiceActivityDetector,
    #[cfg(feature = "voice")]
    whisper: WhisperEngine,
    detected: bool,
}

impl WakeWordDetector {
    pub fn new(wake_word: &str, _vad_threshold: f32, _whisper_model: &str) -> Self {
        Self {
            wake_word: wake_word.to_lowercase(),
            #[cfg(feature = "voice")]
            vad: VoiceActivityDetector::new(_vad_threshold),
            #[cfg(feature = "voice")]
            whisper: WhisperEngine::new(_whisper_model, true),
            detected: false,
        }
    }

    /// Process a raw audio frame through VAD → Whisper micro-transcription.
    /// Returns `true` when the wake word is detected in a speech segment.
    #[cfg(feature = "voice")]
    pub fn process_frame(
        &mut self,
        frame: &AudioFrame,
    ) -> Result<bool, VoiceError> {
        let segment = self.vad.process(frame)?;
        let Some(segment) = segment else {
            return Ok(false);
        };

        // Micro-transcribe the short speech burst.
        let transcription = self.whisper.transcribe(&segment)?;
        let found = self.check_text(&transcription.text)?;
        if found {
            self.detected = true;
        }
        Ok(found)
    }

    /// Check transcribed text for the wake word (case-insensitive substring).
    /// Exposed for pipeline use when transcription is already available.
    pub fn check_text(&mut self, text: &str) -> Result<bool, VoiceError> {
        let normalised = text.to_lowercase();
        Ok(normalised.contains(&self.wake_word))
    }

    /// Reset detection state and internal VAD.
    pub fn reset(&mut self) {
        self.detected = false;
        #[cfg(feature = "voice")]
        self.vad.reset();
    }

    pub fn is_detected(&self) -> bool {
        self.detected
    }

    pub fn wake_word(&self) -> &str {
        &self.wake_word
    }
}
