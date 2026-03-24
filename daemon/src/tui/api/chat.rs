// Chat API — create sessions and send messages to /api/chat/*

use reqwest::Client;
use serde_json::Value;

/// POST /api/chat/session — creates a new chat session, returns session_id.
/// Returns None on any error.
pub async fn create_session(client: &Client, api_url: &str) -> Option<String> {
    let url = format!("{api_url}/api/chat/session");
    let resp = client
        .post(&url)
        .json(&serde_json::json!({}))
        .send()
        .await
        .ok()?;
    let val: Value = resp.json().await.ok()?;
    val.get("session_id")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}

/// POST /api/chat/message — sends a user message, returns assistant reply content.
/// Returns None on any error.
pub async fn send_message(
    client: &Client,
    api_url: &str,
    session_id: &str,
    content: &str,
) -> Option<String> {
    let url = format!("{api_url}/api/chat/message");
    let body = serde_json::json!({
        "session_id": session_id,
        "content": content,
        "role": "user"
    });
    let resp = client.post(&url).json(&body).send().await.ok()?;
    let val: Value = resp.json().await.ok()?;
    // Try common response shapes: {content: "..."} or {message: {content: "..."}}
    val.get("content")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .or_else(|| {
            val.get("message")
                .and_then(|m| m.get("content"))
                .and_then(Value::as_str)
                .map(|s| s.to_string())
        })
}

/// Parse a chat session creation JSON response — extracted for unit testing.
pub fn parse_session_response(val: &Value) -> Option<String> {
    val.get("session_id")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}

/// Parse a chat message JSON response — extracted for unit testing.
pub fn parse_message_response(val: &Value) -> Option<String> {
    val.get("content")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .or_else(|| {
            val.get("message")
                .and_then(|m| m.get("content"))
                .and_then(Value::as_str)
                .map(|s| s.to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_session_response_extracts_session_id() {
        let val = serde_json::json!({"session_id": "sess-abc-123", "ok": true});
        assert_eq!(
            parse_session_response(&val),
            Some("sess-abc-123".to_string())
        );
    }

    #[test]
    fn parse_session_response_returns_none_on_missing_key() {
        let val = serde_json::json!({"ok": true});
        assert_eq!(parse_session_response(&val), None);
    }

    #[test]
    fn parse_message_response_extracts_flat_content() {
        let val = serde_json::json!({
            "content": "Plan 708 is complete.",
            "role": "assistant"
        });
        assert_eq!(
            parse_message_response(&val),
            Some("Plan 708 is complete.".to_string())
        );
    }

    #[test]
    fn parse_message_response_extracts_nested_content() {
        let val = serde_json::json!({
            "message": {
                "content": "3 agents active.",
                "role": "assistant"
            }
        });
        assert_eq!(
            parse_message_response(&val),
            Some("3 agents active.".to_string())
        );
    }

    #[test]
    fn parse_message_response_returns_none_on_empty() {
        let val = serde_json::json!({"ok": true});
        assert_eq!(parse_message_response(&val), None);
    }
}
