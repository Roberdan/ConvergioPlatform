// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// TTS integration for kernel messages — macOS `say` fallback, Voxtral MLX future target.
// Pattern: AppleFmBridge subprocess model (see ipc/models/apple_fm.rs).

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

/// Supported TTS backend strategies.
#[derive(Debug, Clone, PartialEq)]
pub enum TtsBackend {
    /// macOS built-in `say` command — zero deps, supports Italian (Alice voice).
    MacOsSay,
    /// Voxtral MLX (future) — replace `say` when model verified on Apple Silicon.
    VoxtralMlx,
}

/// Kernel message templates in Italian.
pub struct KernelTemplates;

impl KernelTemplates {
    /// "Piano {name} completato. Costo {cost} dollari, durata {duration}."
    pub fn plan_completed(name: &str, cost: &str, duration: &str) -> String {
        format!("Piano {name} completato. Costo {cost} dollari, durata {duration}.")
    }

    /// "Attenzione: il daemon non risponde da {minutes} minuti."
    pub fn daemon_unresponsive(minutes: &str) -> String {
        format!("Attenzione: il daemon non risponde da {minutes} minuti.")
    }

    /// "Task {task_id} bloccato: {reason}."
    pub fn task_blocked(task_id: &str, reason: &str) -> String {
        format!("Task {task_id} bloccato: {reason}.")
    }
}

/// TTS engine — wraps macOS `say` as practical fallback; Voxtral MLX as future target.
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
        let backend = if Self::voxtral_available() {
            TtsBackend::VoxtralMlx
        } else {
            TtsBackend::MacOsSay
        };
        let model_name = match &backend {
            TtsBackend::VoxtralMlx => "voxtral-mlx".to_string(),
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

    /// Returns `true` when the Voxtral MLX CLI is reachable on Apple Silicon.
    pub fn voxtral_available() -> bool {
        // Probe: `python -m voxtral.generate --help` exits 0 when installed.
        std::process::Command::new("python")
            .args(["-m", "voxtral.generate", "--help"])
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
            TtsBackend::MacOsSay => self.speak_via_say(text, locale)?,
            TtsBackend::VoxtralMlx => self.speak_via_voxtral(text, locale)?,
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

    fn speak_via_voxtral(&self, text: &str, _locale: &str) -> Result<Vec<u8>, TtsError> {
        // Placeholder — wire real Voxtral MLX CLI when model is verified.
        // Shape mirrors AppleFmBridge.run_subprocess pattern.
        let wav_path = self.temp_wav_path();
        let output = std::process::Command::new("python")
            .args([
                "-m", "voxtral.generate",
                "--text", text,
                "--output", wav_path.to_str().unwrap_or("/tmp/convergio_tts.wav"),
            ])
            .output()
            .map_err(|e| TtsError::Unavailable(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(TtsError::SubprocessFailed(stderr));
        }
        std::fs::read(&wav_path)
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_init() {
        let e = TtsEngine::new();
        assert!(e.loaded);
        assert!(!e.model_name.is_empty());
    }

    #[test]
    fn test_templates() {
        let p = KernelTemplates::plan_completed("Alpha", "42", "3 ore");
        assert!(p.starts_with("Piano ") && p.contains("Alpha") && p.contains("42"));
        let d = KernelTemplates::daemon_unresponsive("15");
        assert!(d.contains("Attenzione") && d.contains("15"));
        let t = KernelTemplates::task_blocked("T2-01", "dipendenza mancante");
        assert!(t.starts_with("Task ") && t.contains("T2-01") && t.contains("dipendenza"));
    }

    #[test]
    fn test_speak_cache_hit() {
        let mut engine = TtsEngine::new();
        engine.phrase_cache.insert("it-IT:Ciao".to_string(), b"RIFF stub".to_vec());
        let first = engine.speak("Ciao", "it-IT").expect("cache hit");
        let second = engine.speak("Ciao", "it-IT").expect("second cache hit");
        assert_eq!(first, second);
    }

    #[test]
    fn test_locale_differentiates_cache() {
        let mut engine = TtsEngine::new();
        engine.phrase_cache.insert("it-IT:Hello".to_string(), b"it".to_vec());
        engine.phrase_cache.insert("en-US:Hello".to_string(), b"en".to_vec());
        assert_ne!(
            engine.phrase_cache.get("it-IT:Hello"),
            engine.phrase_cache.get("en-US:Hello")
        );
    }

    #[test]
    fn test_backend_detection_no_panic() {
        let _ = TtsEngine::voxtral_available();
        let _ = TtsEngine::say_available();
    }

    #[test]
    fn test_error_display() {
        assert!(TtsError::SubprocessFailed("oops".to_string()).to_string().contains("oops"));
        assert!(TtsError::Unavailable("none".to_string()).to_string().contains("none"));
        assert!(TtsError::Template("bad".to_string()).to_string().contains("bad"));
    }
}
