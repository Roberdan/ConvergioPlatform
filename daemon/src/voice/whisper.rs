use super::types::{SpeechSegment, VoiceError};
use std::path::PathBuf;

/// Transcription result from Whisper STT.
#[derive(Debug, Clone)]
pub struct Transcription {
    pub text: String,
    pub language: String,
    pub confidence: f32,
    pub is_partial: bool,
}

/// Convert i16 PCM samples to f32 normalized [-1.0, 1.0].
pub fn samples_to_f32(samples: &[i16]) -> Vec<f32> {
    samples
        .iter()
        .map(|&s| s as f32 / i16::MAX as f32)
        .collect()
}

/// Resolve the GGML model path for whisper-rs.
/// Searches: $WHISPER_MODEL_PATH, ~/.cache/whisper/, bundled data/.
fn resolve_model_path(model_size: &str) -> PathBuf {
    let filename = format!("ggml-{model_size}.bin");

    // 1. Explicit env override
    if let Ok(p) = std::env::var("WHISPER_MODEL_PATH") {
        let path = PathBuf::from(p);
        if path.exists() {
            return path;
        }
    }

    // 2. Standard cache location
    if let Some(home) = dirs::home_dir() {
        let cache_path = home.join(".cache/whisper").join(&filename);
        if cache_path.exists() {
            return cache_path;
        }
    }

    // 3. Project-local fallback
    let local_path = PathBuf::from("data/models").join(&filename);
    if local_path.exists() {
        return local_path;
    }

    // Return expected path (caller handles missing file)
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cache/whisper")
        .join(filename)
}

/// ASR engine wrapping whisper-rs for native Rust STT.
/// Loads GGML model on first transcription (lazy init).
pub struct WhisperEngine {
    model_size: String,
    prefer_local: bool,
    #[cfg(feature = "voice")]
    ctx: std::sync::Mutex<Option<whisper_rs::WhisperContext>>,
}

impl WhisperEngine {
    pub fn new(model_size: &str, prefer_local: bool) -> Self {
        Self {
            model_size: model_size.to_string(),
            prefer_local,
            #[cfg(feature = "voice")]
            ctx: std::sync::Mutex::new(None),
        }
    }

    pub fn model_size(&self) -> &str {
        &self.model_size
    }

    pub fn prefers_local(&self) -> bool {
        self.prefer_local
    }

    /// Return the resolved model file path for this engine's model size.
    pub fn model_path(&self) -> String {
        resolve_model_path(&self.model_size)
            .to_string_lossy()
            .into_owned()
    }

    /// Transcribe a speech segment using whisper-rs native inference.
    pub fn transcribe(
        &self,
        segment: &SpeechSegment,
    ) -> Result<Transcription, VoiceError> {
        if segment.samples.is_empty() {
            return Err(VoiceError::AsrError(
                "empty audio segment".to_string(),
            ));
        }

        if self.prefer_local {
            self.transcribe_local(segment)
        } else {
            self.transcribe_api(segment)
        }
    }

    /// Native whisper-rs inference with lazy model loading.
    fn transcribe_local(
        &self,
        segment: &SpeechSegment,
    ) -> Result<Transcription, VoiceError> {
        #[cfg(feature = "voice")]
        {
            self.ensure_context_loaded()?;
            let guard = self.ctx.lock().map_err(|e| {
                VoiceError::AsrError(format!("context lock poisoned: {e}"))
            })?;
            let ctx = guard.as_ref().expect("context loaded above");
            let audio_f32 = samples_to_f32(&segment.samples);
            run_whisper_inference(ctx, &audio_f32)
        }

        #[cfg(not(feature = "voice"))]
        {
            drop(segment);
            Err(VoiceError::ModelNotAvailable(
                "whisper-rs model not available: voice feature disabled"
                    .to_string(),
            ))
        }
    }

    /// API fallback (placeholder for external Whisper API).
    fn transcribe_api(
        &self,
        segment: &SpeechSegment,
    ) -> Result<Transcription, VoiceError> {
        drop(segment);
        Err(VoiceError::AsrError(
            "whisper API fallback not yet implemented".to_string(),
        ))
    }

    /// Lazy-init the whisper-rs context from the GGML model file.
    #[cfg(feature = "voice")]
    fn ensure_context_loaded(&self) -> Result<(), VoiceError> {
        let mut guard = self.ctx.lock().map_err(|e| {
            VoiceError::AsrError(format!("context lock poisoned: {e}"))
        })?;
        if guard.is_some() {
            return Ok(());
        }
        let path = resolve_model_path(&self.model_size);
        if !path.exists() {
            return Err(VoiceError::ModelNotAvailable(format!(
                "whisper model not found: {}",
                path.display()
            )));
        }
        let params = whisper_rs::WhisperContextParameters::default();
        let ctx = whisper_rs::WhisperContext::new_with_params(
            path.to_str().unwrap_or_default(),
            params,
        )
        .map_err(|e| {
            VoiceError::AsrError(format!(
                "failed to load whisper model: {e}"
            ))
        })?;
        *guard = Some(ctx);
        Ok(())
    }
}

/// Run inference on an already-loaded whisper-rs context.
#[cfg(feature = "voice")]
fn run_whisper_inference(
    ctx: &whisper_rs::WhisperContext,
    audio: &[f32],
) -> Result<Transcription, VoiceError> {
    use whisper_rs::FullParams;
    use whisper_rs::SamplingStrategy;

    let mut state = ctx.create_state().map_err(|e| {
        VoiceError::AsrError(format!("whisper state error: {e}"))
    })?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_translate(false);
    params.set_no_timestamps(true);
    params.set_single_segment(true);

    state.full(params, audio).map_err(|e| {
        VoiceError::AsrError(format!("whisper inference failed: {e}"))
    })?;

    let n_segments = state.full_n_segments().map_err(|e| {
        VoiceError::AsrError(format!("segment count error: {e}"))
    })?;
    let mut text = String::new();
    for i in 0..n_segments {
        if let Ok(seg_text) = state.full_get_segment_text(i) {
            text.push_str(&seg_text);
        }
    }

    Ok(Transcription {
        text: text.trim().to_string(),
        language: "en".to_string(),
        confidence: 1.0, // whisper-rs greedy doesn't expose per-token probs
        is_partial: false,
    })
}
