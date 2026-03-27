use super::types::VoiceError;

/// Lightweight keyword spotter for wake word detection.
/// Compares transcribed text against the configured wake word.
pub struct WakeWordDetector {
    wake_word: String,
    /// Number of consecutive detections required to trigger.
    trigger_count: u32,
    current_count: u32,
}

impl WakeWordDetector {
    pub fn new(wake_word: &str) -> Self {
        Self {
            wake_word: wake_word.to_lowercase(),
            trigger_count: 1,
            current_count: 0,
        }
    }

    /// Check if the transcribed text contains the wake word.
    pub fn check(&mut self, text: &str) -> Result<bool, VoiceError> {
        let normalised = text.to_lowercase();
        if normalised.contains(&self.wake_word) {
            self.current_count += 1;
            if self.current_count >= self.trigger_count {
                self.current_count = 0;
                return Ok(true);
            }
        } else {
            self.current_count = 0;
        }
        Ok(false)
    }

    /// Reset detection state.
    pub fn reset(&mut self) {
        self.current_count = 0;
    }

    pub fn wake_word(&self) -> &str {
        &self.wake_word
    }
}
