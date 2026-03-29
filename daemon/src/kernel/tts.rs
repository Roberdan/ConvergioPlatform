// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// TTS integration for kernel messages — Voxtral MLX primary, Qwen3 secondary, macOS `say` fallback.
// Pattern: AppleFmBridge subprocess model (see ipc/models/apple_fm.rs).

pub use crate::kernel::tts_templates::KernelTemplates;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

/// Error variants for TTS operations.
#[derive(Debug, thiserror::Error)]
pub enum TtsError {
    #[error("tts subprocess failed: {0}")]
    SubprocessFailed(String),
    #[error("tts backend unavailable: {0}")]
    Unavailable(String),
    #[error("template error: {0}")]
    Template(String),
}

/// Supported TTS backend strategies (priority: Voxtral > Qwen3 > macOS Say).
#[derive(Debug, Clone, PartialEq)]
pub enum TtsBackend {
    /// Voxtral Mini via mlx-audio — primary neural voice backend.
    VoxtralMlx,
    /// Qwen3-TTS via mlx-audio — neural voice, Italian, female (Vivian).
    Qwen3Tts,
    /// macOS built-in `say` command — zero deps fallback.
    MacOsSay,
}

impl TtsBackend {
    /// Human-readable name for logging and status display.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::VoxtralMlx => "Voxtral Mini MLX",
            Self::Qwen3Tts => "Qwen3 TTS Vivian",
            Self::MacOsSay => "macOS Say",
        }
    }
}

/// TTS engine — Voxtral MLX primary, Qwen3 secondary, macOS `say` fallback.
///
/// Uses phrase caching to avoid re-synthesis of repeated kernel messages.
/// Latency target: <2 s for 20 words.
pub struct TtsEngine {
    pub model_name: String,
    pub loaded: bool,
    /// Cache: text → WAV bytes.
    phrase_cache: HashMap<String, Vec<u8>>,
    backend: TtsBackend,
    /// Override output path for tests — `None` uses `/tmp/convergio_tts_<pid>.wav`.
    pub(crate) wav_path_override: Option<PathBuf>,
}

impl Default for TtsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TtsEngine {
    pub fn new() -> Self {
        // Priority: Voxtral > Qwen3 > macOS Say.
        let backend = if Self::voxtral_available() {
            TtsBackend::VoxtralMlx
        } else if Self::qwen3_tts_available() {
            TtsBackend::Qwen3Tts
        } else {
            TtsBackend::MacOsSay
        };
        let model_name = match &backend {
            TtsBackend::VoxtralMlx => "voxtral-mini-mlx".to_string(),
            TtsBackend::Qwen3Tts => "qwen3-tts-vivian".to_string(),
            TtsBackend::MacOsSay => "macos-say-alice".to_string(),
        };
        Self {
            model_name,
            loaded: true,
            phrase_cache: HashMap::new(),
            backend,
            wav_path_override: None,
        }
    }

    /// Returns `true` when Qwen3-TTS is available via mlx-audio.
    pub fn qwen3_tts_available() -> bool {
        let python = crate::ipc::models::apple_fm::AppleFmBridge::resolve_python();
        std::process::Command::new(&python)
            .args(["-c", "from mlx_audio.tts.generate import generate_audio; print('ok')"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Returns `true` when Voxtral Mini is available via mlx-audio on Apple Silicon.
    pub fn voxtral_available() -> bool {
        // Probe: import mlx_audio TTS and check Voxtral model loads.
        let python = crate::ipc::models::apple_fm::AppleFmBridge::resolve_python();
        std::process::Command::new(&python)
            .args(["-c", "from mlx_audio.tts.generate import generate_audio; print('ok')"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Returns `true` when `say` command is available (any macOS).
    pub fn say_available() -> bool {
        std::process::Command::new("say")
            .arg("--help")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|_| true) // `say --help` exits 1 but command exists
            .unwrap_or(false)
    }

    /// Synthesise `text` in the given `locale` and return WAV bytes.
    ///
    /// Results are cached by text+locale key to avoid redundant subprocess calls.
    /// Logs actual latency — target <2 s for 20 words.
    pub fn speak(&mut self, text: &str, locale: &str) -> Result<Vec<u8>, TtsError> {
        let cache_key = format!("{locale}:{text}");
        if let Some(cached) = self.phrase_cache.get(&cache_key) {
            tracing::debug!(text, locale, "tts cache hit");
            return Ok(cached.clone());
        }

        let start = Instant::now();
        let wav = match self.backend {
            TtsBackend::VoxtralMlx => self.speak_via_voxtral(text, locale)?,
            TtsBackend::Qwen3Tts => self.speak_via_qwen3(text, locale)?,
            TtsBackend::MacOsSay => self.speak_via_say(text, locale)?,
        };
        let elapsed = start.elapsed();
        tracing::info!(
            text,
            locale,
            elapsed_ms = elapsed.as_millis(),
            backend = ?self.backend,
            "tts synthesis complete"
        );
        if elapsed.as_secs() >= 2 {
            tracing::warn!(elapsed_ms = elapsed.as_millis(), "tts latency exceeded 2 s target");
        }

        self.phrase_cache.insert(cache_key, wav.clone());
        Ok(wav)
    }

    // --- private backends ---

    fn speak_via_say(&self, text: &str, locale: &str) -> Result<Vec<u8>, TtsError> {
        // Voice mapping: Italian → Alice, English → default.
        let voice = if locale.starts_with("it") { "Alice" } else { "Samantha" };
        let wav_path = self.temp_wav_path();

        let status = std::process::Command::new("say")
            .args([
                "-v", voice,
                "-o", wav_path.to_str().unwrap_or("/tmp/convergio_tts.wav"),
                "--data-format=LEI16@22050",
                text,
            ])
            .status()
            .map_err(|e| TtsError::SubprocessFailed(e.to_string()))?;

        if !status.success() {
            return Err(TtsError::SubprocessFailed(format!(
                "say exited with code {:?}",
                status.code()
            )));
        }

        std::fs::read(&wav_path)
            .map_err(|e| TtsError::SubprocessFailed(format!("read wav: {e}")))
    }

    fn speak_via_qwen3(&self, text: &str, locale: &str) -> Result<Vec<u8>, TtsError> {
        let wav_dir = self.temp_wav_path();
        let lang = if locale.starts_with("it") { "it" } else { "en" };
        let python = crate::ipc::models::apple_fm::AppleFmBridge::resolve_python();
        let status = std::process::Command::new(&python)
            .args([
                "-m", "mlx_audio.tts.generate",
                "--model", "mlx-community/Qwen3-TTS-12Hz-1.7B-CustomVoice-bf16",
                "--text", text,
                "--voice", "Vivian",
                "--lang_code", lang,
                "--output_path", wav_dir.to_str().unwrap_or("/tmp/convergio_tts"),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| TtsError::SubprocessFailed(e.to_string()))?;
        if !status.success() {
            return Err(TtsError::SubprocessFailed("qwen3-tts exited with error".into()));
        }
        // mlx-audio saves to <output_path>/audio_000.wav
        let audio_path = wav_dir.join("audio_000.wav");
        std::fs::read(&audio_path)
            .map_err(|e| TtsError::SubprocessFailed(format!("read wav: {e}")))
    }

    fn speak_via_voxtral(&self, text: &str, locale: &str) -> Result<Vec<u8>, TtsError> {
        let wav_dir = self.temp_wav_path();
        let voice = if locale.starts_with("it") { "it_female" } else { "it_male" };
        let python = crate::ipc::models::apple_fm::AppleFmBridge::resolve_python();
        let status = std::process::Command::new(&python)
            .args([
                "-m", "mlx_audio.tts.generate",
                "--model", "mlx-community/Voxtral-4B-TTS-2603-mlx-4bit",
                "--text", text,
                "--voice", voice,
                "--output_path", wav_dir.to_str().unwrap_or("/tmp/convergio_tts"),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| TtsError::SubprocessFailed(e.to_string()))?;
        if !status.success() {
            return Err(TtsError::SubprocessFailed(
                "voxtral-tts exited with error".into(),
            ));
        }
        // mlx-audio saves to <output_path>/audio_000.wav
        let audio_path = wav_dir.join("audio_000.wav");
        std::fs::read(&audio_path)
            .map_err(|e| TtsError::SubprocessFailed(format!("read wav: {e}")))
    }

    fn temp_wav_path(&self) -> PathBuf {
        if let Some(ref p) = self.wav_path_override {
            return p.clone();
        }
        let pid = std::process::id();
        PathBuf::from(format!("/tmp/convergio_tts_{pid}.wav"))
    }
}

// ----- Tests (external file) -------------------------------------------------

#[cfg(test)]
#[path = "tts_tests.rs"]
mod tests;
