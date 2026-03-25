// Why: Plan 725 T3-02 — Email adapter implementing ChannelAdapter via HTTP relay.
// Uses webhook-to-email pattern (Mailgun/SendGrid API) — no SMTP crate needed.

use super::{AsyncChannelResult, ChannelAdapter, ChannelError, ChannelHealth, ChannelMessage};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;

/// Action extracted from a `[CONVERGIO-ACTION-{id}]` subject marker.
#[derive(Debug, PartialEq)]
pub enum SubjectAction {
    Approve(String),
    Reject(String),
}

/// Parse `[CONVERGIO-APPROVE-{id}]` or `[CONVERGIO-REJECT-{id}]` from a subject line.
/// Returns `None` if neither marker is present or the format is invalid.
pub fn parse_subject_action(subject: &str) -> Option<SubjectAction> {
    // Scan for the bracketed marker anywhere in the subject
    let start = subject.find('[')? + 1;
    let end = subject[start..].find(']')? + start;
    let tag = &subject[start..end];

    if let Some(id) = tag.strip_prefix("CONVERGIO-APPROVE-") {
        if !id.is_empty() {
            return Some(SubjectAction::Approve(id.into()));
        }
    }
    if let Some(id) = tag.strip_prefix("CONVERGIO-REJECT-") {
        if !id.is_empty() {
            return Some(SubjectAction::Reject(id.into()));
        }
    }
    None
}

/// JSON payload sent to the HTTP relay endpoint.
#[derive(Debug, Serialize)]
struct EmailPayload {
    from: String,
    to: String,
    subject: String,
    text: String,
}

/// Email channel adapter — dispatches via a configurable HTTP relay.
/// Inbound polling is a stub; call `poll_inbox` against an endpoint returning
/// JSON `[{ "subject": "...", "text": "..." }]`.
pub struct EmailAdapter {
    endpoint: String,
    from: String,
    to: String,
    client: reqwest::Client,
    connected: bool,
    last_message_at: Option<DateTime<Utc>>,
    error_count: u64,
}

impl EmailAdapter {
    /// Build from environment variables.
    pub fn from_env() -> Result<Self, ChannelError> {
        let endpoint = std::env::var("CONVERGIO_EMAIL_ENDPOINT").map_err(|_| {
            ChannelError::ConnectionFailed("CONVERGIO_EMAIL_ENDPOINT not set".into())
        })?;
        let from = std::env::var("CONVERGIO_EMAIL_FROM").map_err(|_| {
            ChannelError::ConnectionFailed("CONVERGIO_EMAIL_FROM not set".into())
        })?;
        let to = std::env::var("CONVERGIO_EMAIL_TO").map_err(|_| {
            ChannelError::ConnectionFailed("CONVERGIO_EMAIL_TO not set".into())
        })?;
        Ok(Self::new_with_config(endpoint, from, to))
    }

    /// Build with explicit config (used in tests and direct construction).
    pub fn new_with_config(endpoint: String, from: String, to: String) -> Self {
        Self {
            endpoint,
            from,
            to,
            client: reqwest::Client::new(),
            connected: false,
            last_message_at: None,
            error_count: 0,
        }
    }

    /// Validate config fields before marking connected.
    fn validate_config(&self) -> Result<(), ChannelError> {
        if self.endpoint.is_empty() {
            return Err(ChannelError::ConnectionFailed(
                "CONVERGIO_EMAIL_ENDPOINT must not be empty".into(),
            ));
        }
        if self.from.is_empty() {
            return Err(ChannelError::ConnectionFailed(
                "CONVERGIO_EMAIL_FROM must not be empty".into(),
            ));
        }
        if self.to.is_empty() {
            return Err(ChannelError::ConnectionFailed(
                "CONVERGIO_EMAIL_TO must not be empty".into(),
            ));
        }
        Ok(())
    }

    /// Build the outbound email payload from a ChannelMessage.
    fn build_payload(&self, msg: &ChannelMessage) -> EmailPayload {
        let subject = msg
            .metadata
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("Convergio Notification")
            .to_string();
        EmailPayload {
            from: self.from.clone(),
            to: self.to.clone(),
            subject,
            text: msg.content.clone(),
        }
    }

    /// Poll an HTTP inbox endpoint for new inbound messages.
    /// Endpoint must return `[{"subject": "...", "text": "..."}]`.
    /// Subject actions are extracted and returned as ChannelMessages.
    pub async fn poll_inbox(&mut self, inbox_url: &str) -> Result<Vec<ChannelMessage>, ChannelError> {
        let resp = self.client.get(inbox_url).send().await?;
        if !resp.status().is_success() {
            return Err(ChannelError::Other(format!(
                "inbox poll returned {}",
                resp.status()
            )));
        }
        let items: Vec<serde_json::Value> = resp.json().await?;
        let mut messages = Vec::new();
        for item in items {
            let subject = item["subject"].as_str().unwrap_or("").to_string();
            let text = item["text"].as_str().unwrap_or("").to_string();
            self.last_message_at = Some(Utc::now());
            messages.push(ChannelMessage {
                id: format!("email-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
                source_channel: "email".into(),
                content: text,
                reply_to: None,
                metadata: serde_json::json!({"subject": subject}),
                timestamp: Utc::now(),
            });
        }
        Ok(messages)
    }
}

impl ChannelAdapter for EmailAdapter {
    fn connect<'a>(&'a mut self) -> AsyncChannelResult<'a, ()> {
        Box::pin(async move {
            self.validate_config()?;
            self.connected = true;
            Ok(())
        })
    }

    fn send<'a>(&'a self, msg: &'a ChannelMessage) -> AsyncChannelResult<'a, ()> {
        Box::pin(async move {
            let payload = self.build_payload(msg);
            let resp = self
                .client
                .post(&self.endpoint)
                .json(&payload)
                .send()
                .await?;
            if !resp.status().is_success() {
                return Err(ChannelError::Other(format!(
                    "relay returned HTTP {}",
                    resp.status()
                )));
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

    fn health<'a>(&'a self) -> Pin<Box<dyn Future<Output = ChannelHealth> + Send + 'a>> {
        Box::pin(async move {
            ChannelHealth {
                connected: self.connected,
                last_message_at: self.last_message_at,
                error_count: self.error_count,
                channel_name: "email".into(),
            }
        })
    }

    fn name(&self) -> &str {
        "email"
    }
}

#[cfg(test)]
#[path = "email_tests.rs"]
mod tests;
