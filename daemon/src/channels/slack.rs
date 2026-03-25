// Why: Plan 725 T3-01 — Slack adapter implementing ChannelAdapter trait.
// Uses Slack Web API (chat.postMessage) and auth.test for token validation.
// Socket Mode WebSocket listener is a placeholder; real deployment activates it.

use super::{AsyncChannelResult, ChannelAdapter, ChannelError, ChannelHealth, ChannelMessage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

/// Slack Web API adapter for Convergio channel integration.
#[derive(Debug)]
pub struct SlackAdapter {
    bot_token: String,
    channel_id: String,
    client: reqwest::Client,
    base_url: String,
    connected: bool,
    last_message_at: Option<DateTime<Utc>>,
    error_count: u64,
}

/// Slack API top-level response envelope.
#[derive(Debug, Deserialize)]
struct SlackResponse {
    ok: bool,
    error: Option<String>,
}

/// chat.postMessage request body.
#[derive(Debug, Serialize)]
struct PostMessageBody {
    channel: String,
    text: String,
}

impl SlackAdapter {
    /// Create adapter from environment variables.
    pub fn from_env() -> Result<Self, ChannelError> {
        let token = std::env::var("CONVERGIO_SLACK_BOT_TOKEN")
            .map_err(|_| ChannelError::AuthFailed("CONVERGIO_SLACK_BOT_TOKEN not set".into()))?;
        let channel = std::env::var("CONVERGIO_SLACK_CHANNEL")
            .map_err(|_| ChannelError::Other("CONVERGIO_SLACK_CHANNEL not set".into()))?;
        Self::new(token, channel)
    }

    /// Create adapter with explicit credentials. Validates token is non-empty.
    pub fn new(bot_token: String, channel_id: String) -> Result<Self, ChannelError> {
        if bot_token.is_empty() {
            return Err(ChannelError::AuthFailed("bot token must not be empty".into()));
        }
        Ok(Self::new_with_base_url(
            bot_token,
            channel_id,
            "https://slack.com".into(),
        ))
    }

    /// Create adapter with custom base URL (for testing).
    pub fn new_with_base_url(bot_token: String, channel_id: String, base_url: String) -> Self {
        Self {
            bot_token,
            channel_id,
            client: reqwest::Client::new(),
            base_url,
            connected: false,
            last_message_at: None,
            error_count: 0,
        }
    }

    /// Whether the adapter is currently connected.
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Test-only: force connection state without network call.
    #[cfg(test)]
    pub fn set_connected_for_test(&mut self, value: bool) {
        self.connected = value;
    }

    /// Build Authorization header value.
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.bot_token)
    }

    /// Build full API URL for a given Slack method path.
    fn api_url(&self, method_path: &str) -> String {
        format!("{}/{}", self.base_url, method_path)
    }
}

impl ChannelAdapter for SlackAdapter {
    fn connect<'a>(&'a mut self) -> AsyncChannelResult<'a, ()> {
        Box::pin(async move {
            let url = self.api_url("api/auth.test");
            let resp: SlackResponse = self
                .client
                .post(&url)
                .header("Authorization", self.auth_header())
                .send()
                .await?
                .json()
                .await?;

            if !resp.ok {
                let msg = resp.error.unwrap_or_else(|| "auth.test failed".into());
                return Err(ChannelError::AuthFailed(msg));
            }
            self.connected = true;
            Ok(())
        })
    }

    fn send<'a>(&'a self, msg: &'a ChannelMessage) -> AsyncChannelResult<'a, ()> {
        Box::pin(async move {
            let url = self.api_url("api/chat.postMessage");
            let body = PostMessageBody {
                channel: self.channel_id.clone(),
                text: msg.content.clone(),
            };
            let resp: SlackResponse = self
                .client
                .post(&url)
                .header("Authorization", self.auth_header())
                .json(&body)
                .send()
                .await?
                .json()
                .await?;

            if !resp.ok {
                let msg = resp.error.unwrap_or_else(|| "chat.postMessage failed".into());
                return Err(ChannelError::Other(msg));
            }
            Ok(())
        })
    }

    fn disconnect<'a>(&'a mut self) -> AsyncChannelResult<'a, ()> {
        Box::pin(async move {
            self.connected = false;
            Ok(())
        })
    }

    fn health<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = ChannelHealth> + Send + 'a>> {
        Box::pin(async move {
            ChannelHealth {
                connected: self.connected,
                last_message_at: self.last_message_at,
                error_count: self.error_count,
                channel_name: "slack".into(),
            }
        })
    }

    fn name(&self) -> &str {
        "slack"
    }
}

/// Parse a Slack slash command text into (command_name, optional_arg).
/// Returns None if the command is not a recognized Convergio command.
pub fn parse_slash_command(text: &str) -> Option<(String, Option<String>)> {
    let trimmed = text.trim();
    let known = ["convergio-status", "convergio-approve"];
    for &cmd in &known {
        let prefix = format!("/{}", cmd);
        if trimmed == prefix {
            return Some((cmd.into(), None));
        }
        let with_space = format!("{} ", prefix);
        if let Some(rest) = trimmed.strip_prefix(&with_space) {
            let arg = rest.trim();
            return Some((cmd.into(), if arg.is_empty() { None } else { Some(arg.into()) }));
        }
    }
    None
}

/// Format a human-readable response for a recognized slash command.
pub fn format_command_response(command: &str, arg: Option<&str>) -> String {
    match command {
        "convergio-status" => "Convergio status: all systems operational".into(),
        "convergio-approve" => {
            if let Some(id) = arg {
                format!("Approval requested for task {}", id)
            } else {
                "Usage: /convergio-approve <task-id>".into()
            }
        }
        _ => format!("Unknown command: {}", command),
    }
}

#[cfg(test)]
#[path = "slack_tests.rs"]
mod tests;
