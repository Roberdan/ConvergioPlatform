use super::types::VoiceError;
use std::process::Command;

/// Text-to-Speech engine with system TTS fallback.
/// Uses macOS `say` command as reliable fallback.
pub struct TtsEngine {
    voice: String,
    rate: f32,
}

impl TtsEngine {
    pub fn new(voice: &str, rate: f32) -> Self {
        Self {
            voice: voice.to_string(),
            rate: rate.clamp(0.5, 2.0),
        }
    }

    /// Speak text using the configured TTS engine.
    /// Falls back to macOS `say` command.
    pub fn speak(&self, text: &str) -> Result<(), VoiceError> {
        self.speak_system(text)
    }

    /// Speak using macOS system TTS (say command).
    fn speak_system(&self, text: &str) -> Result<(), VoiceError> {
        let rate_wpm = (self.rate * 175.0) as u32;
        let mut cmd = Command::new("say");
        cmd.arg("-r").arg(rate_wpm.to_string());
        if self.voice != "default" {
            cmd.arg("-v").arg(&self.voice);
        }
        cmd.arg(text);
        cmd.output()
            .map_err(|e| VoiceError::TtsError(format!("say command failed: {e}")))?;
        Ok(())
    }

    /// Check if system TTS is available.
    pub fn is_available() -> bool {
        Command::new("which")
            .arg("say")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn voice(&self) -> &str {
        &self.voice
    }
}
