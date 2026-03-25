// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// F-28: Notification channels — ntfy.sh, Telegram, macOS.
// Config loaded from claude-config/config/notifications.conf.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use std::time::Duration;
use tracing::warn;

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
        // Prefer terminal-notifier; fall back to osascript.
        let tn = std::process::Command::new("terminal-notifier")
            .args(["-title", &msg.title, "-message", &msg.message, "-sound", "default"])
            .status();
        if let Ok(s) = tn {
            if s.success() { return Ok(()); }
        }
        let script = format!("display notification {:?} with title {:?}", msg.message, msg.title);
        std::process::Command::new("osascript")
            .args(["-e", &script])
            .status()
            .map(|_| ())
            .map_err(|e| format!("osascript failed: {e}"))
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

/// Dispatch to all configured channels; log individual failures, do not abort.
pub async fn dispatch(channels: &[ChannelConfig], msg: &NotifyMessage) {
    for ch in channels {
        let result = match ch {
            ChannelConfig::Ntfy { topic, base_url } => {
                NtfyChannel::new(topic, base_url).send(msg).await
            }
            ChannelConfig::Telegram { bot_token, chat_id } => {
                TelegramChannel::new(bot_token, chat_id).send(msg).await
            }
            ChannelConfig::MacOS => MacOSChannel.send(msg).await,
        };
        if let Err(e) = result {
            warn!("notify dispatch error: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_from_str_critical() {
        assert!(matches!("critical".parse::<NotifySeverity>().unwrap(), NotifySeverity::Critical));
    }

    #[test]
    fn severity_from_str_warning() {
        assert!(matches!("warning".parse::<NotifySeverity>().unwrap(), NotifySeverity::Warning));
    }

    #[test]
    fn severity_from_str_info() {
        assert!(matches!("info".parse::<NotifySeverity>().unwrap(), NotifySeverity::Info));
    }

    #[test]
    fn severity_from_str_invalid() {
        assert!("unknown".parse::<NotifySeverity>().is_err());
    }

    #[test]
    fn severity_display() {
        assert_eq!(format!("{}", NotifySeverity::Critical), "critical");
        assert_eq!(format!("{}", NotifySeverity::Warning), "warning");
        assert_eq!(format!("{}", NotifySeverity::Info), "info");
    }

    #[test]
    fn severity_priority_order() {
        assert!(NotifySeverity::Critical.priority() > NotifySeverity::Warning.priority());
        assert!(NotifySeverity::Warning.priority() > NotifySeverity::Info.priority());
    }

    #[test]
    fn notify_message_builds() {
        let msg = NotifyMessage {
            title: "Test".to_string(),
            message: "hello".to_string(),
            severity: NotifySeverity::Info,
        };
        assert_eq!(msg.title, "Test");
    }
}
