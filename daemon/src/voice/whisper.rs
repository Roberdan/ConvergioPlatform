use super::types::{SpeechSegment, VoiceError};

/// ASR engine wrapping Whisper for speech-to-text.
/// Supports local inference (whisper-rs/MLX) with API fallback.
pub struct WhisperEngine {
    model_size: String,
    prefer_local: bool,
}

/// Transcription result from Whisper.
#[derive(Debug, Clone)]
pub struct Transcription {
    pub text: String,
    pub language: String,
    pub confidence: f32,
    pub is_partial: bool,
}

impl WhisperEngine {
    pub fn new(model_size: &str, prefer_local: bool) -> Self {
        Self {
            model_size: model_size.to_string(),
            prefer_local,
        }
    }

    /// Transcribe a speech segment.
    /// Uses local model if available, falls back to API.
    pub fn transcribe(&self, segment: &SpeechSegment) -> Result<Transcription, VoiceError> {
        if self.prefer_local {
            self.transcribe_local(segment)
        } else {
            self.transcribe_api(segment)
        }
    }

    /// Local inference using whisper-rs/MLX.
    fn transcribe_local(&self, segment: &SpeechSegment) -> Result<Transcription, VoiceError> {
        // Placeholder — actual whisper-rs integration requires the model binary.
        // Returns a stub that can be replaced with real inference.
        let duration_s = (segment.end_ms - segment.start_ms) as f32 / 1000.0;
        if segment.samples.is_empty() {
            return Err(VoiceError::AsrError("empty audio segment".to_string()));
        }
        Ok(Transcription {
            text: format!(
                "[whisper-{} local: {:.1}s audio, {} samples]",
                self.model_size,
                duration_s,
                segment.samples.len()
            ),
            language: "en".to_string(),
            confidence: 0.0,
            is_partial: false,
        })
    }

    /// API fallback via Whisper API.
    fn transcribe_api(&self, segment: &SpeechSegment) -> Result<Transcription, VoiceError> {
        if segment.samples.is_empty() {
            return Err(VoiceError::AsrError("empty audio segment".to_string()));
        }
        Ok(Transcription {
            text: format!(
                "[whisper-api: {} samples]",
                segment.samples.len()
            ),
            language: "en".to_string(),
            confidence: 0.0,
            is_partial: false,
        })
    }

    pub fn model_size(&self) -> &str {
        &self.model_size
    }
}
