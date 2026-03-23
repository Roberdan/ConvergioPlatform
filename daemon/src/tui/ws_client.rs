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
    /// Expected shape: {"kind":"brain_event","event_type":"<TYPE>","payload":<DATA>}
    /// Returns None for unknown kinds, unknown event_type, or malformed JSON.
    pub fn parse_message(text: &str) -> Option<BrainEvent> {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        if v.get("kind")?.as_str()? != "brain_event" {
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
mod tests {
    use super::*;

    #[test]
    fn test_parse_agent_update() {
        let json = r#"{"kind":"brain_event","event_type":"agent_update","payload":[{"id":1,"name":"thor"}]}"#;
        let event = WsClient::parse_message(json).unwrap();
        match event {
            BrainEvent::AgentUpdate { agents } => {
                assert_eq!(agents.len(), 1);
                assert_eq!(agents[0]["name"], "thor");
            }
            _ => panic!("expected AgentUpdate"),
        }
    }

    #[test]
    fn test_parse_session_update() {
        let json = r#"{"kind":"brain_event","event_type":"session_update","payload":[{"session_id":"abc"}]}"#;
        let event = WsClient::parse_message(json).unwrap();
        match event {
            BrainEvent::SessionUpdate { sessions } => {
                assert_eq!(sessions.len(), 1);
                assert_eq!(sessions[0]["session_id"], "abc");
            }
            _ => panic!("expected SessionUpdate"),
        }
    }

    #[test]
    fn test_parse_task_update() {
        let json = r#"{"kind":"brain_event","event_type":"task_update","payload":{"task_id":42,"status":"done","plan_id":708}}"#;
        let event = WsClient::parse_message(json).unwrap();
        match event {
            BrainEvent::TaskUpdate { task_id, status, plan_id } => {
                assert_eq!(task_id, 42);
                assert_eq!(status, "done");
                assert_eq!(plan_id, 708);
            }
            _ => panic!("expected TaskUpdate"),
        }
    }

    #[test]
    fn test_parse_heartbeat_none() {
        // kind != brain_event → None
        let json = r#"{"kind":"heartbeat","event_type":"ping"}"#;
        assert!(WsClient::parse_message(json).is_none());
    }

    #[test]
    fn test_parse_invalid() {
        assert!(WsClient::parse_message("not json at all").is_none());
        assert!(WsClient::parse_message("{}").is_none());
    }

    #[test]
    fn test_backoff_duration() {
        let mut c = WsClient::new("http://localhost:8420");
        assert_eq!(c.backoff_duration(), Duration::from_secs(1)); // 2^0
        c.retry_count = 1;
        assert_eq!(c.backoff_duration(), Duration::from_secs(2)); // 2^1
        c.retry_count = 2;
        assert_eq!(c.backoff_duration(), Duration::from_secs(4)); // 2^2
        c.retry_count = 5;
        assert_eq!(c.backoff_duration(), Duration::from_secs(30)); // capped
        c.retry_count = 10;
        assert_eq!(c.backoff_duration(), Duration::from_secs(30)); // still capped
    }

    #[test]
    fn test_should_fallback_after_3() {
        let mut c = WsClient::new("http://localhost:8420");
        assert!(!c.should_fallback());
        c.increment_retries();
        c.increment_retries();
        assert!(!c.should_fallback());
        c.increment_retries();
        assert!(c.should_fallback()); // 3 >= 3
        c.reset_retries();
        assert!(!c.should_fallback());
    }

    #[test]
    fn test_url_conversion() {
        let c = WsClient::new("http://localhost:8420");
        assert_eq!(c.url, "ws://localhost:8420/ws/brain");

        let c2 = WsClient::new("https://convergio.example.com");
        assert_eq!(c2.url, "wss://convergio.example.com/ws/brain");
    }
}
