// Why: Plan 725 T1-02 — ChannelAdapter trait for bidirectional external channel integration.
// Follows Pin<Box<dyn Future>> pattern (same as workspace/git_connector.rs) for dyn-compatibility.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

/// Boxed async result for trait methods — enables dyn ChannelAdapter without async_trait crate.
pub type AsyncChannelResult<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ChannelError>> + Send + 'a>>;

/// Message flowing through a channel (inbound or outbound).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    pub id: String,
    pub source_channel: String,
    pub content: String,
    pub reply_to: Option<String>,
    pub metadata: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

/// Health status of a channel adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelHealth {
    pub connected: bool,
    pub last_message_at: Option<DateTime<Utc>>,
    pub error_count: u64,
    pub channel_name: String,
}

/// Errors that channel adapters can produce.
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

/// Bidirectional channel adapter trait for external service integration.
/// Object-safe: uses Pin<Box<Future>> instead of async fn in trait.
pub trait ChannelAdapter: Send + Sync {
    /// Connect to the external service.
    fn connect<'a>(&'a mut self) -> AsyncChannelResult<'a, ()>;
    /// Send a message through the channel.
    fn send<'a>(&'a self, msg: &'a ChannelMessage) -> AsyncChannelResult<'a, ()>;
    /// Disconnect from the external service.
    fn disconnect<'a>(&'a mut self) -> AsyncChannelResult<'a, ()>;
    /// Get current health status.
    fn health<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = ChannelHealth> + Send + 'a>>;
    /// Channel name identifier.
    fn name(&self) -> &str;
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
