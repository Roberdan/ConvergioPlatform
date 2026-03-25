use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

pub type AsyncChannelResult<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ChannelError>> + Send + 'a>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    pub id: String,
    pub source_channel: String,
    pub content: String,
    pub reply_to: Option<String>,
    pub metadata: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelHealth {
    pub connected: bool,
    pub last_message_at: Option<DateTime<Utc>>,
    pub error_count: u64,
    pub channel_name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("rate limited: retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
    #[error("authentication failed: {0}")]
    AuthFailed(String),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("channel error: {0}")]
    Other(String),
}

pub trait ChannelAdapter: Send + Sync {
    fn connect<'a>(&'a mut self) -> AsyncChannelResult<'a, ()>;
    fn send<'a>(&'a self, msg: &'a ChannelMessage) -> AsyncChannelResult<'a, ()>;
    fn disconnect<'a>(&'a mut self) -> AsyncChannelResult<'a, ()>;
    fn health<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = ChannelHealth> + Send + 'a>>;
    fn name(&self) -> &str;
}

pub mod email;
pub mod router;
pub mod slack;
pub mod telegram;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
