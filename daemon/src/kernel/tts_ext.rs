// TTS Voxtral backend — separated from tts.rs for 250-line limit.

use std::path::PathBuf;
use super::tts::{TtsEngine, TtsError};

impl TtsEngine {
    pub(crate) fn speak_via_voxtral(&self, text: &str, locale: &str) -> Result<Vec<u8>, TtsError> {
        let wav_dir = self.temp_wav_path();
        let _ = locale; // all locales use the same voice — casual_female sounds best
        let voice = "casual_female";
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

    pub(crate) fn temp_wav_path(&self) -> PathBuf {
        if let Some(ref p) = self.wav_path_override {
            return p.clone();
        }
        let pid = std::process::id();
        PathBuf::from(format!("/tmp/convergio_tts_{pid}.wav"))
    }
}
