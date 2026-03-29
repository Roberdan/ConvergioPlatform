// Why: Plan 725 T2-01 — Telegram adapter implementing ChannelAdapter trait.
// Uses Telegram Bot API with long-polling for bidirectional messaging.

use super::telegram_types::{
    SendMessageBody, TelegramResponse, TelegramUpdate, TelegramUser,
};
use super::{AsyncChannelResult, ChannelAdapter, ChannelError, ChannelHealth, ChannelMessage};
use chrono::{DateTime, Utc};
use std::future::Future;
use std::pin::Pin;

/// Telegram Bot API adapter for Convergio channel integration.
pub struct TelegramAdapter {
    bot_token: String,
    chat_id: Option<i64>,
    client: reqwest::Client,
    base_url: String,
    last_update_id: i64,
    connected: bool,
    last_message_at: Option<DateTime<Utc>>,
    error_count: u64,
}

impl TelegramAdapter {
    /// Create adapter from environment variable.
    pub fn from_env(chat_id: Option<i64>) -> Result<Self, ChannelError> {
        let token = std::env::var("CONVERGIO_TELEGRAM_TOKEN").map_err(|_| {
            ChannelError::AuthFailed("CONVERGIO_TELEGRAM_TOKEN not set".into())
        })?;
        Ok(Self::new(token, chat_id))
    }

    /// Create adapter with explicit token.
    pub fn new(bot_token: String, chat_id: Option<i64>) -> Self {
        Self::new_with_base_url(bot_token, chat_id, "https://api.telegram.org".into())
    }

    /// Create adapter with custom base URL (for testing).
    pub fn new_with_base_url(bot_token: String, chat_id: Option<i64>, base_url: String) -> Self {
        Self {
            bot_token,
            chat_id,
            client: reqwest::Client::new(),
            base_url,
            last_update_id: 0,
            connected: false,
            last_message_at: None,
            error_count: 0,
        }
    }

    /// Whether the adapter is currently connected.
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Build API URL for a given method.
    fn api_url(&self, method_name: &str) -> String {
        format!("{}/bot{}/{}", self.base_url, self.bot_token, method_name)
    }

    /// Poll for new messages via getUpdates long-polling.
    pub async fn poll_updates(&mut self) -> Result<Vec<ChannelMessage>, ChannelError> {
        let url = self.api_url("getUpdates");
        let body = serde_json::json!({
            "offset": self.last_update_id + 1,
            "timeout": 30
        });
        let resp: TelegramResponse<Vec<TelegramUpdate>> = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        if !resp.ok {
            return Err(ChannelError::Other(
                resp.description.unwrap_or_else(|| "getUpdates failed".into()),
            ));
        }
        let updates = resp.result.unwrap_or_default();
        let mut messages = Vec::new();
        for update in updates {
            if update.update_id > self.last_update_id {
                self.last_update_id = update.update_id;
            }
            if let Some(msg) = update.message {
                if let Some(text) = msg.text {
                    self.last_message_at = Some(Utc::now());
                    // Auto-learn chat_id from first incoming message
                    if self.chat_id.is_none() {
                        self.chat_id = Some(msg.chat.id);
                    }
                    messages.push(ChannelMessage {
                        id: format!("tg-{}", msg.message_id),
                        source_channel: "telegram".into(),
                        content: text,
                        reply_to: None,
                        metadata: serde_json::json!({
                            "chat_id": msg.chat.id,
                            "from": msg.from.as_ref().map(|u| &u.first_name),
                        }),
                        timestamp: Utc::now(),
                    });
                }
            }
        }
        Ok(messages)
    }
}

impl ChannelAdapter for TelegramAdapter {
    fn connect<'a>(&'a mut self) -> AsyncChannelResult<'a, ()> {
        Box::pin(async move {
            let url = self.api_url("getMe");
            let resp = self.client.post(&url).send().await?;
            if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
                return Err(ChannelError::AuthFailed("invalid bot token".into()));
            }
            let body: TelegramResponse<TelegramUser> = resp.json().await?;
            if !body.ok {
                return Err(ChannelError::ConnectionFailed(
                    body.description.unwrap_or_else(|| "getMe failed".into()),
                ));
            }
            self.connected = true;
            Ok(())
        })
    }

    fn send<'a>(&'a self, msg: &'a ChannelMessage) -> AsyncChannelResult<'a, ()> {
        Box::pin(async move {
            let chat_id = self.chat_id.ok_or_else(|| {
                ChannelError::Other("no chat_id configured — send a message first".into())
            })?;
            let url = self.api_url("sendMessage");
            let body = SendMessageBody {
                chat_id,
                text: msg.content.clone(),
                parse_mode: "Markdown".into(),
            };
            let resp: TelegramResponse<serde_json::Value> =
                self.client.post(&url).json(&body).send().await?.json().await?;
            if !resp.ok {
                return Err(ChannelError::Other(
                    resp.description.unwrap_or_else(|| "sendMessage failed".into()),
                ));
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
                channel_name: "telegram".into(),
            }
        })
    }

    fn name(&self) -> &str {
        "telegram"
    }
}

/// Route a command string to the appropriate handler. Returns response text.
pub fn route_command(text: &str, _agents: &[&str]) -> String {
    let trimmed = text.trim();
    if trimmed == "/status" {
        "Convergio status: all systems operational".into()
    } else if let Some(id) = trimmed.strip_prefix("/approve ") {
        format!("Approval requested for task {}", id.trim())
    } else if let Some(id) = trimmed.strip_prefix("/reject ") {
        format!("Rejection requested for task {}", id.trim())
    } else if let Some(query) = trimmed.strip_prefix("/ask ") {
        format!("Routing query to orchestrator: {query}")
    } else {
        "Available commands:\n/status — system status\n\
         /approve <id> — approve a task\n\
         /reject <id> — reject a task\n\
         /ask <query> — ask the orchestrator"
            .into()
    }
}

#[cfg(test)]
#[path = "telegram_tests.rs"]
mod tests;
