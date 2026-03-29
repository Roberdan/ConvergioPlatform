// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// STT (Speech-to-Text) integration — Whisper via mlx_whisper or whisper CLI subprocess.
// Privacy: audio bytes never stored permanently, never sent to cloud. Only text is logged.
// Follows the AppleFmBridge pattern for subprocess isolation.

use serde::{Deserialize, Serialize};
use std::io::Write as _;
use std::time::Duration;

/// Result of a single transcription call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transcription {
    /// Transcribed text.
    pub text: String,
    /// BCP-47 locale detected by the model (e.g. "en", "it"). Empty if not detected.
    pub locale: String,
    /// Confidence in [0.0, 1.0]. 0.0 when not reported by the CLI.
    pub confidence: f32,
}

/// Errors from the STT engine.
#[derive(Debug, thiserror::Error)]
pub enum SttError {
    #[error("STT model not loaded: {0}")]
    ModelNotLoaded(String),
    #[error("STT subprocess failed: {0}")]
    SubprocessFailed(String),
    #[error("STT timeout after {0:?}")]
    Timeout(Duration),
    #[error("STT unavailable: {0}")]
    Unavailable(String),
    #[error("STT io error: {0}")]
    Io(String),
}

/// Whisper STT engine. Shells out to `python -m mlx_whisper` or `whisper-cpp` CLI.
///
/// Degrades gracefully when no CLI is installed: `transcribe` returns
/// `SttError::Unavailable` so callers surface the degraded state explicitly
/// (Fail-Loud principle — silent `return null` is a BUG).
pub struct SttEngine {
    pub model_name: String,
    pub loaded: bool,
    /// Override CLI path for tests; `None` → resolved from PATH.
    pub(crate) cli_override: Option<String>,
}

impl Default for SttEngine {
    fn default() -> Self {
        Self {
            model_name: "whisper-small".to_string(),
            loaded: false,
            cli_override: None,
        }
    }
}

impl SttEngine {
    /// Create a new engine with `whisper-small` model (not yet loaded).
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark the engine as loaded (call after verifying CLI availability).
    pub fn load(&mut self) {
        self.loaded = true;
    }

    /// Returns `true` when a usable Whisper CLI is reachable on PATH.
    pub fn is_available(&self) -> bool {
        if let Some(path) = &self.cli_override {
            return std::path::Path::new(path).exists();
        }
        self.mlx_whisper_available() || self.whisper_cpp_available()
    }

    /// Transcribe raw audio bytes (WAV/MP3/any format whisper accepts).
    ///
    /// Bytes are written to a temp file; whisper CLI processes it; the temp
    /// file is deleted immediately — audio never persists beyond the call.
    pub fn transcribe(&self, audio_bytes: &[u8]) -> Result<Transcription, SttError> {
        if !self.loaded {
            return Err(SttError::ModelNotLoaded(
                "call load() or ensure model is initialized before transcribe()".to_string(),
            ));
        }
        if audio_bytes.is_empty() {
            return Err(SttError::Unavailable("empty audio input".to_string()));
        }
        let tmp = self
            .write_tmp_audio(audio_bytes)
            .map_err(|e| SttError::Io(e.to_string()))?;
        let result = self.run_whisper(&tmp);
        // Privacy: delete temp audio immediately.
        if let Err(e) = std::fs::remove_file(&tmp) {
            tracing::debug!("stt: temp audio cleanup: {e}");
        }
        result
    }

    // --- private helpers ---

    fn mlx_whisper_available(&self) -> bool {
        std::process::Command::new("python")
            .args(["-m", "mlx_whisper", "--help"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn whisper_cpp_available(&self) -> bool {
        std::process::Command::new("whisper-cpp")
            .arg("--help")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Build the base argv tokens for the whisper subprocess.
    fn cli_argv(&self) -> Vec<String> {
        if let Some(path) = &self.cli_override {
            return vec![path.clone()];
        }
        if self.mlx_whisper_available() {
            vec![
                "python".to_string(),
                "-m".to_string(),
                "mlx_whisper".to_string(),
            ]
        } else {
            vec!["whisper-cpp".to_string()]
        }
    }

    fn write_tmp_audio(&self, bytes: &[u8]) -> std::io::Result<String> {
        let path = format!("/tmp/cvg_stt_{}.wav", std::process::id());
        let mut f = std::fs::File::create(&path)?;
        f.write_all(bytes)?;
        Ok(path)
    }

    fn run_whisper(&self, audio_path: &str) -> Result<Transcription, SttError> {
        let argv = self.cli_argv();
        let timeout = Duration::from_secs(60);

        let child = std::process::Command::new(&argv[0])
            .args(&argv[1..])
            .args(["--model", &self.model_name, "--output-format", "txt", audio_path])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| SttError::SubprocessFailed(e.to_string()))?;

        // Enforce wall-clock timeout via background thread (mirrors AppleFmBridge).
        let child_id = child.id();
        let _timeout_guard = std::thread::spawn(move || {
            std::thread::sleep(timeout);
            #[cfg(unix)]
            kill_process(child_id);
        });

        let output = child
            .wait_with_output()
            .map_err(|e| SttError::SubprocessFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr.contains("killed") || stderr.contains("timeout") {
                return Err(SttError::Timeout(timeout));
            }
            return Err(SttError::SubprocessFailed(stderr));
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let locale =
            parse_locale_from_stderr(&String::from_utf8_lossy(&output.stderr));

        Ok(Transcription {
            text,
            locale,
            // mlx_whisper does not expose per-segment confidence in plain-txt mode
            confidence: 0.9,
        })
    }
}

/// Parse the BCP-47 locale from whisper stderr (e.g. "Detected language: en").
pub(crate) fn parse_locale_from_stderr(stderr: &str) -> String {
    for line in stderr.lines() {
        let lower = line.to_lowercase();
        if lower.contains("detected language:") {
            if let Some(lang) = line.split(':').nth(1) {
                return lang.trim().to_lowercase();
            }
        }
    }
    String::new()
}

/// Send SIGTERM to a child process on Unix (mirrors AppleFmBridge).
#[cfg(unix)]
fn kill_process(pid: u32) {
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    unsafe {
        kill(pid as i32, 15 /* SIGTERM */);
    }
}

// ---------------------------------------------------------------------------
// API request/response types (used by kernel/api.rs)
// ---------------------------------------------------------------------------

/// Response for POST /api/kernel/transcribe and POST /api/kernel/listen.
#[derive(Debug, Serialize, Deserialize)]
pub struct TranscribeResponse {
    pub text: String,
    pub locale: String,
    pub confidence: f32,
}

impl From<Transcription> for TranscribeResponse {
    fn from(t: Transcription) -> Self {
        Self {
            text: t.text,
            locale: t.locale,
            confidence: t.confidence,
        }
    }
}
