// WebSocket client for TUI real-time updates — struct, parsing, backoff logic.
// Actual WS connection/stream wiring is deferred to W3/W4.

use std::time::Duration;

/// Events received from the /ws/brain WebSocket endpoint.
#[derive(Debug, PartialEq)]
pub enum BrainEvent {
    AgentUpdate { agents: Vec<serde_json::Value> },
    SessionUpdate { sessions: Vec<serde_json::Value> },
    TaskUpdate { task_id: i64, status: String, plan_id: i64 },
    Heartbeat,
    /// Bulk snapshot sent by the server on initial WS connection.
    /// Used to populate the brain canvas with the current state before
    /// incremental updates arrive — without this the canvas starts blank.
    HeartbeatSnapshot { peers: Vec<serde_json::Value> },
}

/// WebSocket client with exponential backoff and HTTP fallback support.
pub struct WsClient {
    pub url: String,
    pub auth_token: Option<String>,
    pub retry_count: u32,
    pub max_retries_before_fallback: u32,
}

impl WsClient {
    /// Convert http://host:port → ws://host:port/ws/brain.
    /// Reads CONVERGIO_AUTH_TOKEN env var for optional auth.
    pub fn new(api_url: &str) -> Self {
        let url = api_url
            .replacen("https://", "wss://", 1)
            .replacen("http://", "ws://", 1);
        let url = format!("{}/ws/brain", url.trim_end_matches('/'));
        let auth_token = std::env::var("CONVERGIO_AUTH_TOKEN").ok();
        Self {
            url,
            auth_token,
            retry_count: 0,
            max_retries_before_fallback: 3,
        }
    }

    /// Parse a JSON text frame into a BrainEvent.
    ///
    /// Accepted shapes:
    /// - `{"kind":"brain_event","event_type":"<TYPE>","payload":<DATA>}`
    /// - `{"kind":"heartbeat_snapshot","peers":[...]}`  (initial canvas seed)
    ///
    /// Returns None for unknown kinds/event_types or malformed JSON.
    pub fn parse_message(text: &str) -> Option<BrainEvent> {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        let kind = v.get("kind")?.as_str()?;

        // heartbeat_snapshot is a top-level message kind (not wrapped in brain_event)
        // sent immediately after the WS handshake to seed the initial brain canvas state.
        if kind == "heartbeat_snapshot" {
            let peers = v
                .get("peers")
                .and_then(|p| p.as_array())
                .cloned()
                .unwrap_or_default();
            return Some(BrainEvent::HeartbeatSnapshot { peers });
        }

        if kind != "brain_event" {
            return None;
        }
        let event_type = v.get("event_type")?.as_str()?;
        let payload = v.get("payload").cloned().unwrap_or(serde_json::Value::Null);

        match event_type {
            "agent_update" => {
                let agents = payload
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                Some(BrainEvent::AgentUpdate { agents })
            }
            "session_update" => {
                let sessions = payload
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                Some(BrainEvent::SessionUpdate { sessions })
            }
            "task_update" => {
                let task_id = payload.get("task_id")?.as_i64()?;
                let status = payload.get("status")?.as_str()?.to_string();
                let plan_id = payload.get("plan_id")?.as_i64()?;
                Some(BrainEvent::TaskUpdate { task_id, status, plan_id })
            }
            "heartbeat" => Some(BrainEvent::Heartbeat),
            _ => None,
        }
    }

    /// Exponential backoff: min(1s × 2^retry_count, 30s).
    pub fn backoff_duration(&self) -> Duration {
        let secs = (1u64 << self.retry_count.min(5)).min(30);
        Duration::from_secs(secs)
    }

    /// True when retries have exceeded max — caller should fall back to HTTP polling.
    pub fn should_fallback(&self) -> bool {
        self.retry_count >= self.max_retries_before_fallback
    }

    pub fn reset_retries(&mut self) {
        self.retry_count = 0;
    }

    pub fn increment_retries(&mut self) {
        self.retry_count = self.retry_count.saturating_add(1);
    }
}

#[cfg(test)]
#[path = "ws_client_tests.rs"]
mod tests;
