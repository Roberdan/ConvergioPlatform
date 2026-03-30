// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Kernel outbound Telegram notifications: text (sendMessage) + voice (sendVoice OGG).
// Builds on channels/telegram.rs adapter — reuses TelegramAdapter::new_with_base_url.
// Config prefers CONVERGIO_TELEGRAM_* with TELEGRAM_* legacy aliases supported.

use crate::channels::telegram::TelegramAdapter;
use crate::channels::ChannelAdapter;
use crate::channels::ChannelMessage;
use crate::kernel::recover::Severity;
use chrono::{Timelike, Utc};
use reqwest::multipart;
use serde::Deserialize;
use tracing::{info, warn};

// ----- Public types ----------------------------------------------------------

/// Outbound notification routing mode (env: KERNEL_NOTIFY).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotifyMode {
    /// Telegram only (default).
    Telegram,
    /// Local audio via active mesh node.
    Local,
    /// Both Telegram and local audio.
    Both,
}

impl NotifyMode {
    /// Resolve from KERNEL_NOTIFY env var; default: Telegram.
    pub fn from_env() -> Self {
        match std::env::var("KERNEL_NOTIFY").as_deref() {
            Ok("local") => Self::Local,
            Ok("both") => Self::Both,
            _ => Self::Telegram,
        }
    }
}

/// Quiet hours window (env: KERNEL_QUIET_HOURS e.g. "23:00-07:00" CET).
/// During quiet hours CRITICAL → Telegram text only (no voice).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuietHoursConfig {
    pub start_hour: u8,
    pub start_minute: u8,
    pub end_hour: u8,
    pub end_minute: u8,
}

impl QuietHoursConfig {
    /// Parse "HH:MM-HH:MM" format; returns None on invalid input.
    pub fn parse(raw: &str) -> Option<Self> {
        let (start, end) = raw.split_once('-')?;
        let (sh, sm) = start.split_once(':')?;
        let (eh, em) = end.split_once(':')?;
        Some(Self {
            start_hour: match sh.parse() { Ok(v) => v, Err(_) => return None },
            start_minute: match sm.parse() { Ok(v) => v, Err(_) => return None },
            end_hour: match eh.parse() { Ok(v) => v, Err(_) => return None },
            end_minute: match em.parse() { Ok(v) => v, Err(_) => return None },
        })
    }

    /// Load from KERNEL_QUIET_HOURS env var.
    pub fn from_env() -> Option<Self> {
        match std::env::var("KERNEL_QUIET_HOURS") {
            Ok(v) => Self::parse(&v),
            Err(_) => None,
        }
    }

    /// Returns true if (hour, minute) falls inside the quiet window (wraps midnight).
    pub fn is_active_at(&self, hour: u8, minute: u8) -> bool {
        let now_mins = u16::from(hour) * 60 + u16::from(minute);
        let start_mins = u16::from(self.start_hour) * 60 + u16::from(self.start_minute);
        let end_mins = u16::from(self.end_hour) * 60 + u16::from(self.end_minute);
        if start_mins < end_mins {
            // Normal same-day range (e.g. 09:00-17:00)
            now_mins >= start_mins && now_mins < end_mins
        } else {
            // Wraps midnight (e.g. 23:00-07:00)
            now_mins >= start_mins || now_mins < end_mins
        }
    }

    /// Returns true if current UTC time falls inside the quiet window.
    pub fn is_active_now(&self) -> bool {
        let now = Utc::now();
        // Why: quiet hours defined in CET (+1/+2) but kernel runs UTC.
        // For simplicity, compare UTC; operator should set KERNEL_QUIET_HOURS in UTC offset.
        self.is_active_at(now.hour() as u8, now.minute() as u8)
    }
}

// ----- Telegram API response -------------------------------------------------

#[derive(Deserialize)]
struct TgResponse {
    ok: bool,
    description: Option<String>,
}

// ----- Public API ------------------------------------------------------------

/// Send a Markdown-formatted text message via Telegram sendMessage.
///
/// `base_url` overrides "https://api.telegram.org" (for testing).
pub async fn send_text(
    token: &str,
    chat_id: i64,
    text: &str,
    base_url: Option<&str>,
) -> Result<(), String> {
    let adapter = TelegramAdapter::new_with_base_url(
        token.to_owned(),
        Some(chat_id),
        base_url.unwrap_or("https://api.telegram.org").to_owned(),
    );
    let msg = ChannelMessage {
        id: format!("kernel-{}", Utc::now().timestamp_millis()),
        source_channel: "kernel".into(),
        content: text.to_owned(),
        reply_to: None,
        metadata: serde_json::json!({}),
        timestamp: Utc::now(),
    };
    adapter.send(&msg).await.map_err(|e| e.to_string())
}

/// Send OGG Opus audio bytes via Telegram sendVoice (multipart POST).
///
/// `base_url` overrides "https://api.telegram.org" (for testing).
pub async fn send_voice(
    token: &str,
    chat_id: i64,
    audio_bytes: &[u8],
    base_url: Option<&str>,
) -> Result<(), String> {
    let api_base = base_url.unwrap_or("https://api.telegram.org");
    let url = format!("{api_base}/bot{token}/sendVoice");

    let part = multipart::Part::bytes(audio_bytes.to_vec())
        .file_name("voice.ogg")
        .mime_str("audio/ogg")
        .map_err(|e| format!("mime error: {e}"))?;

    let form = multipart::Form::new()
        .text("chat_id", chat_id.to_string())
        .part("voice", part);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("client build: {e}"))?;

    let resp = client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("sendVoice HTTP: {e}"))?;

    let status = resp.status();
    let body: TgResponse = resp.json().await.map_err(|e| format!("sendVoice parse: {e}"))?;
    if !body.ok {
        return Err(format!(
            "sendVoice failed ({}): {}",
            status,
            body.description.unwrap_or_default()
        ));
    }
    Ok(())
}

// ----- Channel routing -------------------------------------------------------

/// Route a kernel message to the appropriate channel based on config and quiet hours.
///
/// `dry_run = true` → skip all I/O, used in tests.
///
/// Routing logic:
/// - KERNEL_NOTIFY=telegram|both → send_text (voice in non-quiet hours)
/// - KERNEL_NOTIFY=local|both → audio via active node (not during quiet hours)
/// - CRITICAL → BOTH channels regardless of KERNEL_NOTIFY
/// - Quiet hours → Telegram text only (no voice, no local audio)
pub async fn communicate(
    message: &str,
    severity: Severity,
    dry_run: bool,
) -> Result<(), String> {
    if dry_run {
        info!("jarvis.telegram: [dry_run] communicate skipped — severity={severity} msg={message}");
        return Ok(());
    }

    let mode = NotifyMode::from_env();
    let quiet = QuietHoursConfig::from_env().map(|q| q.is_active_now()).unwrap_or(false);
    let token = crate::telegram_config::telegram_token();
    let chat_id: Option<i64> = match crate::telegram_config::telegram_chat_id() {
        Ok(value) => value,
        Err(e) => {
            warn!("jarvis.telegram: Telegram chat id parse error: {e}");
            None
        }
    };

    let use_telegram = matches!(mode, NotifyMode::Telegram | NotifyMode::Both)
        || severity == Severity::Critical;
    let use_local = (matches!(mode, NotifyMode::Local | NotifyMode::Both)
        || severity == Severity::Critical)
        && !quiet;

    // Send Telegram text
    if use_telegram {
        match (token.as_deref(), chat_id) {
            (Some(tok), Some(cid)) => {
                // Quiet hours → text only; otherwise also send voice if TTS available
                if let Err(e) = send_text(tok, cid, message, None).await {
                    warn!("jarvis.telegram: sendMessage failed: {e}");
                } else {
                    info!("jarvis.telegram: sendMessage ok");
                }
            }
            _ => warn!(
                "jarvis.telegram: Telegram credentials not set (need CONVERGIO_TELEGRAM_* or TELEGRAM_*)"
            ),
        }
    }

    // Local audio (skipped during quiet hours)
    if use_local {
        let mut tts = crate::kernel::tts::TtsEngine::new();
        if let Ok(audio) = tts.speak(message, "it-IT") {
            crate::kernel::audio::play_local(&audio).await;
        } else {
            warn!("jarvis.telegram: TTS failed for local audio");
        }
    }

    Ok(())
}

// ----- Tests (external file) ------------------------------------------------

#[cfg(test)]
#[path = "telegram_tests.rs"]
mod tests;
