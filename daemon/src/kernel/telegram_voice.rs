// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Telegram inbound voice handler — OGG download → WAV → transcribe → route → reply.
// Privacy: OGG stored only in tempfile (RAII, auto-deleted on drop and on crash).

use crate::kernel::engine::KernelEngine;
use crate::kernel::stt::TranscribeResponse;
use crate::kernel::telegram::{send_text, send_voice};
use crate::kernel::voice_router::{classify_intent, route_intent};
use serde::Deserialize;
use serde_json::Value;
use std::io::Write as _;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tracing::{info, warn};

// ----- Public types ----------------------------------------------------------

/// Minimal voice message envelope extracted from a Telegram update.
#[derive(Debug, Clone)]
pub struct VoiceMessage {
    pub chat_id: i64,
    pub file_id: String,
    pub duration_secs: u32,
}

#[derive(Deserialize)]
struct GetFileResponse {
    ok: bool,
    result: Option<GetFileResult>,
}

#[derive(Deserialize)]
struct GetFileResult {
    file_path: String,
}

// ----- URL helpers (public for tests) ----------------------------------------

/// Build the getFile API URL for a given file_id.
pub fn build_get_file_url(token: &str, file_id: &str, base_url: &str) -> String {
    format!("{base_url}/bot{token}/getFile?file_id={file_id}")
}

/// Build the file download URL from a Telegram file_path.
pub fn build_download_url(token: &str, file_path: &str, base_url: &str) -> String {
    format!("{base_url}/file/bot{token}/{file_path}")
}

// ----- JSON extraction helpers (public for tests) ----------------------------

/// Extract VoiceMessage from a raw Telegram update JSON (no chat filter).
pub fn extract_voice_file_id(update: &Value) -> Option<VoiceMessage> {
    let msg = update.get("message")?;
    let chat_id = msg.get("chat")?.get("id")?.as_i64()?;
    let voice = msg.get("voice")?;
    let file_id = voice.get("file_id")?.as_str()?.to_string();
    let duration_secs = voice.get("duration").and_then(|d| d.as_u64()).unwrap_or(0) as u32;
    Some(VoiceMessage { chat_id, file_id, duration_secs })
}

/// Extract VoiceMessage and apply the authorised chat_id security filter.
pub fn extract_voice_file_id_for_chat(update: &Value, authorised_chat_id: i64) -> Option<VoiceMessage> {
    let vm = extract_voice_file_id(update)?;
    if vm.chat_id != authorised_chat_id { return None; }
    Some(vm)
}

// ----- Core pipeline ---------------------------------------------------------

/// Full voice pipeline: getFile → download OGG (tempfile RAII) → ffmpeg WAV →
/// transcribe → classify/route → reply text + voice note.
pub async fn handle_voice_message(
    token: &str,
    voice: &VoiceMessage,
    daemon_url: &str,
    engine: &Arc<KernelEngine>,
    base_url: Option<&str>,
) -> Result<(), String> {
    let api_base = base_url.unwrap_or("https://api.telegram.org");
    let file_path = get_telegram_file_path(token, &voice.file_id, api_base).await?;
    let ogg_bytes = download_file(token, &file_path, api_base).await?;
    info!("telegram_voice: downloaded OGG {} bytes for file_id={}", ogg_bytes.len(), voice.file_id);
    let wav_bytes = convert_ogg_to_wav(&ogg_bytes).await?;
    let transcript = transcribe_audio(daemon_url, &wav_bytes).await?;
    info!("telegram_voice: transcribed: {:?}", transcript.text);
    let intent = classify_intent(&transcript.text, engine);
    let reply_text = route_intent(intent, daemon_url);
    if let Err(e) = send_text(token, voice.chat_id, &reply_text, base_url).await {
        warn!("telegram_voice: send_text failed: {e}");
    }
    match synthesise_reply(&reply_text) {
        Ok(voice_bytes) => {
            if let Err(e) = send_voice(token, voice.chat_id, &voice_bytes, base_url).await {
                warn!("telegram_voice: send_voice failed: {e}");
            }
        }
        Err(e) => warn!("telegram_voice: TTS failed — text-only reply sent: {e}"),
    }
    Ok(())
}

// ----- Internal helpers ------------------------------------------------------

async fn get_telegram_file_path(token: &str, file_id: &str, api_base: &str) -> Result<String, String> {
    let url = build_get_file_url(token, file_id, api_base);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("client build: {e}"))?;
    let resp: GetFileResponse = client
        .get(&url).send().await.map_err(|e| format!("getFile HTTP: {e}"))?
        .json().await.map_err(|e| format!("getFile parse: {e}"))?;
    if !resp.ok { return Err("getFile returned ok=false".to_string()); }
    resp.result.map(|r| r.file_path).ok_or_else(|| "getFile: missing result.file_path".to_string())
}

async fn download_file(token: &str, file_path: &str, api_base: &str) -> Result<Vec<u8>, String> {
    let url = build_download_url(token, file_path, api_base);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("client build: {e}"))?;
    let bytes = client
        .get(&url).send().await.map_err(|e| format!("download HTTP: {e}"))?
        .bytes().await.map_err(|e| format!("download bytes: {e}"))?;
    Ok(bytes.to_vec())
}

/// Resolve the absolute path to ffmpeg.
/// On macOS/Homebrew the daemon may run under launchd with a minimal PATH that
/// omits /opt/homebrew/bin, so we probe known locations before falling back to PATH.
// pub(crate) so telegram_voice_tests.rs (included via #[path]) can call it.
pub(crate) fn resolve_ffmpeg() -> Result<std::path::PathBuf, String> {
    let candidates = [
        "/opt/homebrew/bin/ffmpeg",
        "/usr/local/bin/ffmpeg",
        "/usr/bin/ffmpeg",
        "ffmpeg",
    ];
    for candidate in &candidates {
        let path = std::path::Path::new(candidate);
        if path.is_absolute() {
            if path.exists() { return Ok(path.to_path_buf()); }
        } else {
            return Ok(path.to_path_buf());
        }
    }
    Err("ffmpeg not found; install via: brew install ffmpeg".to_string())
}

/// Convert OGG Opus bytes → WAV bytes via ffmpeg subprocess.
/// Uses `tempfile::NamedTempFile` for RAII auto-cleanup of both input and output.
async fn convert_ogg_to_wav(ogg_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let ffmpeg = resolve_ffmpeg()?;
    let mut ogg_tmp = tempfile::NamedTempFile::new().map_err(|e| format!("tempfile ogg: {e}"))?;
    ogg_tmp.write_all(ogg_bytes).map_err(|e| format!("write ogg: {e}"))?;
    let ogg_path = ogg_tmp.path().to_path_buf();
    let wav_tmp = tempfile::NamedTempFile::new().map_err(|e| format!("tempfile wav: {e}"))?;
    let wav_path = wav_tmp.path().to_path_buf();
    let status = Command::new(&ffmpeg)
        .args(["-y", "-i", ogg_path.to_str().unwrap_or_default(),
               "-ar", "16000", "-ac", "1", "-f", "wav",
               wav_path.to_str().unwrap_or_default()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map_err(|e| format!("ffmpeg spawn ({}): {e}", ffmpeg.display()))?;
    if !status.success() {
        return Err(format!("ffmpeg exited with {status} ({})", ffmpeg.display()));
    }
    let wav_bytes = tokio::fs::read(&wav_path).await.map_err(|e| format!("read wav: {e}"))?;
    Ok(wav_bytes)
}

/// POST WAV bytes to the kernel transcribe endpoint; returns transcript.
async fn transcribe_audio(daemon_url: &str, wav_bytes: &[u8]) -> Result<TranscribeResponse, String> {
    let url = format!("{daemon_url}/api/kernel/transcribe");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("client build: {e}"))?;
    let part = reqwest::multipart::Part::bytes(wav_bytes.to_vec())
        .file_name("audio.wav").mime_str("audio/wav").map_err(|e| format!("mime: {e}"))?;
    let form = reqwest::multipart::Form::new().part("audio", part);
    let resp: TranscribeResponse = client
        .post(&url).multipart(form).send().await.map_err(|e| format!("transcribe HTTP: {e}"))?
        .json().await.map_err(|e| format!("transcribe parse: {e}"))?;
    Ok(resp)
}

/// Generate TTS OGG bytes for the reply using the configured TTS engine.
fn synthesise_reply(text: &str) -> Result<Vec<u8>, String> {
    let mut tts = crate::kernel::tts::TtsEngine::new();
    tts.speak(text, "it-IT").map_err(|e| e.to_string())
}

// ----- Tests (external file) -------------------------------------------------

#[cfg(test)]
#[path = "telegram_voice_tests.rs"]
mod tests;
