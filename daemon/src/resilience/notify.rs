// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// F-28: Notification channels — ntfy.sh, Telegram, macOS.
// Config loaded from claude-config/config/notifications.conf.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use std::time::Duration;
use std::time::Instant;

/// Notification severity — controls priority in ntfy and Telegram subjects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotifySeverity {
    Critical,
    Warning,
    Info,
}

impl NotifySeverity {
    /// Numeric priority (higher = more urgent).
    pub fn priority(&self) -> u8 {
        match self { Self::Critical => 5, Self::Warning => 3, Self::Info => 1 }
    }
}

impl fmt::Display for NotifySeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Critical => write!(f, "critical"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
        }
    }
}

impl FromStr for NotifySeverity {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "critical" => Ok(Self::Critical),
            "warning" => Ok(Self::Warning),
            "info" => Ok(Self::Info),
            other => Err(format!("unknown severity: {other}")),
        }
    }
}

/// A notification to be dispatched via one or more channels.
#[derive(Debug, Clone)]
pub struct NotifyMessage {
    pub title: String,
    pub message: String,
    pub severity: NotifySeverity,
}

// ─── ntfy.sh ───────────────────────────────────────────────────────────────

pub struct NtfyChannel {
    pub topic: String,
    pub base_url: String,
}

impl NtfyChannel {
    pub fn new(topic: &str, base_url: &str) -> Self {
        Self { topic: topic.to_string(), base_url: base_url.to_string() }
    }

    pub async fn send(&self, msg: &NotifyMessage) -> Result<(), String> {
        let client = Client::builder().timeout(Duration::from_secs(10)).build().unwrap_or_default();
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), self.topic);
        let resp = client
            .post(&url)
            .header("Title", &msg.title)
            .header("Priority", msg.severity.priority().to_string())
            .body(msg.message.clone())
            .send()
            .await
            .map_err(|e| format!("ntfy send failed: {e}"))?;
        if resp.status().is_success() { Ok(()) }
        else { Err(format!("ntfy HTTP {}", resp.status())) }
    }
}

// ─── Telegram ──────────────────────────────────────────────────────────────

pub struct TelegramChannel {
    pub bot_token: String,
    pub chat_id: String,
}

impl TelegramChannel {
    pub fn new(bot_token: &str, chat_id: &str) -> Self {
        Self { bot_token: bot_token.to_string(), chat_id: chat_id.to_string() }
    }

    pub async fn send(&self, msg: &NotifyMessage) -> Result<(), String> {
        let client = Client::builder().timeout(Duration::from_secs(10)).build().unwrap_or_default();
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": format!("[{}] {}: {}", msg.severity, msg.title, msg.message),
        });
        let resp = client.post(&url).json(&body).send().await
            .map_err(|e| format!("telegram send failed: {e}"))?;
        if resp.status().is_success() { Ok(()) }
        else { Err(format!("telegram HTTP {}", resp.status())) }
    }
}

// ─── macOS ─────────────────────────────────────────────────────────────────

pub struct MacOSChannel;

impl MacOSChannel {
    pub async fn send(&self, msg: &NotifyMessage) -> Result<(), String> {
        let status = std::process::Command::new("terminal-notifier")
            .args(["-title", &msg.title, "-message", &msg.message, "-sound", "default"])
            .status()
            .map_err(|e| format!("terminal-notifier launch failed: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("terminal-notifier exited with code {}", status))
        }
    }
}

// ─── Multi-channel dispatcher ──────────────────────────────────────────────

/// Configured channel variants — drives dispatch without trait objects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelConfig {
    Ntfy { topic: String, base_url: String },
    Telegram { bot_token: String, chat_id: String },
    MacOS,
}

/// Per-channel delivery result for fail-loud reporting.
#[derive(Debug, Clone, Serialize)]
pub struct ChannelResult {
    pub channel: String,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Dispatch to all configured channels; return per-channel results.
/// Every failure is logged via tracing::error! and included in the response.
pub async fn dispatch(channels: &[ChannelConfig], msg: &NotifyMessage) -> Vec<ChannelResult> {
    let mut results = Vec::with_capacity(channels.len());
    for ch in channels {
        let started_at = Instant::now();
        let (channel_name, result) = match ch {
            ChannelConfig::Ntfy { topic, base_url } => {
                ("ntfy".to_string(), NtfyChannel::new(topic, base_url).send(msg).await)
            }
            ChannelConfig::Telegram { bot_token, chat_id } => {
                ("telegram".to_string(), TelegramChannel::new(bot_token, chat_id).send(msg).await)
            }
            ChannelConfig::MacOS => {
                ("macos".to_string(), MacOSChannel.send(msg).await)
            }
        };
        let duration_ms = started_at.elapsed().as_millis() as u64;
        match result {
            Ok(()) => results.push(ChannelResult {
                channel: channel_name,
                success: true,
                error: None,
                duration_ms,
            }),
            Err(e) => {
                tracing::error!(channel = %channel_name, error = %e, "notification delivery failed");
                results.push(ChannelResult {
                    channel: channel_name,
                    success: false,
                    error: Some(e),
                    duration_ms,
                });
            }
        }
    }
    results
}

#[cfg(test)]
#[path = "notify_dispatch_tests.rs"]
mod tests;
