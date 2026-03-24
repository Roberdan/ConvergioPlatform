// Chat API — send messages via daemon mesh delegate/exec, with /api/chat/* fallback.
// Why delegate over /api/chat/message: no LLM backend there; mesh/exec runs claude CLI.

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
    // Try: {session_id: "..."} or {session: {id: "..."}} or {ok: true, session: {id: "..."}}
    val.get("session_id").and_then(Value::as_str).map(String::from)
        .or_else(|| val.get("session").and_then(|s| s.get("id")).and_then(Value::as_str).map(String::from))
}

/// POST /api/mesh/delegate (primary) → /api/mesh/exec (fallback) → helpful error.
///
/// Why: /api/chat/message has no LLM backend. Running `claude --print` via mesh
/// delegate executes the locally logged-in claude CLI session — no API key needed.
pub async fn send_message(
    client: &Client,
    api_url: &str,
    _session_id: &str,
    content: &str,
) -> Option<String> {
    // Escape single quotes in user input to avoid shell injection in the command string.
    let safe_content = content.replace('\'', "'\\''");
    let command = format!("claude --print '{safe_content}'");

    let body = serde_json::json!({
        "command": command,
        "node": "local",
        "timeout_secs": 60
    });

    // Try primary: /api/mesh/delegate
    if let Some(reply) = try_delegate(client, api_url, &body).await {
        return Some(reply);
    }

    // Try fallback: /api/mesh/exec
    if let Some(reply) = try_exec(client, api_url, &body).await {
        return Some(reply);
    }

    // Both failed — return a helpful message rather than silent None.
    Some("Chat unavailable — daemon mesh/delegate not responding".to_string())
}

/// Attempt POST /api/mesh/delegate and extract stdout from response.
async fn try_delegate(client: &Client, api_url: &str, body: &Value) -> Option<String> {
    let url = format!("{api_url}/api/mesh/delegate");
    let resp = client
        .post(&url)
        .json(body)
        .timeout(std::time::Duration::from_secs(65))
        .send()
        .await
        .ok()?;
    // 404 or non-2xx means endpoint doesn't support this body format → try fallback
    if !resp.status().is_success() {
        return None;
    }
    let val: Value = resp.json().await.ok()?;
    parse_delegate_response(&val)
}

/// Attempt POST /api/mesh/exec and extract stdout from response.
async fn try_exec(client: &Client, api_url: &str, body: &Value) -> Option<String> {
    let url = format!("{api_url}/api/mesh/exec");
    let resp = client
        .post(&url)
        .json(body)
        .timeout(std::time::Duration::from_secs(65))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let val: Value = resp.json().await.ok()?;
    parse_delegate_response(&val)
}

/// Parse a delegate/exec JSON response — looks for stdout in:
///
/// - `{stdout: "..."}` directly
/// - `{result: {stdout: "..."}}` (exec wraps result)
///
/// Returns None if stdout is absent or empty.
pub fn parse_delegate_response(val: &Value) -> Option<String> {
    // Direct stdout field
    if let Some(s) = val.get("stdout").and_then(Value::as_str) {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    // Nested under result (exec endpoint wraps response)
    if let Some(s) = val
        .get("result")
        .and_then(|r| r.get("stdout"))
        .and_then(Value::as_str)
    {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

/// Parse a chat session creation JSON response — extracted for unit testing.
pub fn parse_session_response(val: &Value) -> Option<String> {
    val.get("session_id").and_then(Value::as_str).map(String::from)
        .or_else(|| val.get("session").and_then(|s| s.get("id")).and_then(Value::as_str).map(String::from))
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

    // --- parse_delegate_response tests (new behavior) ---

    #[test]
    fn parse_delegate_response_extracts_direct_stdout() {
        let val = serde_json::json!({"ok": true, "stdout": "3 active plans", "stderr": "", "exit_code": 0});
        assert_eq!(
            parse_delegate_response(&val),
            Some("3 active plans".to_string())
        );
    }

    #[test]
    fn parse_delegate_response_extracts_nested_stdout_under_result() {
        // /api/mesh/exec wraps the inner response under "result"
        let val = serde_json::json!({
            "ok": true, "peer": "local", "method": "ssh",
            "result": {"stdout": "Hello from claude", "exit_code": 0}
        });
        assert_eq!(
            parse_delegate_response(&val),
            Some("Hello from claude".to_string())
        );
    }

    #[test]
    fn parse_delegate_response_returns_none_on_empty_stdout() {
        let val = serde_json::json!({"ok": true, "stdout": "", "exit_code": 0});
        assert_eq!(parse_delegate_response(&val), None);
    }

    #[test]
    fn parse_delegate_response_returns_none_on_whitespace_only_stdout() {
        let val = serde_json::json!({"ok": true, "stdout": "   \n  ", "exit_code": 0});
        assert_eq!(parse_delegate_response(&val), None);
    }

    #[test]
    fn parse_delegate_response_trims_stdout_whitespace() {
        let val = serde_json::json!({"ok": true, "stdout": "  answer\n", "exit_code": 0});
        assert_eq!(
            parse_delegate_response(&val),
            Some("answer".to_string())
        );
    }

    #[test]
    fn parse_delegate_response_returns_none_on_missing_stdout() {
        let val = serde_json::json!({"ok": true, "delegated_to": "linux-worker"});
        assert_eq!(parse_delegate_response(&val), None);
    }

    // --- existing parse_session_response tests ---

    #[test]
    fn parse_session_response_extracts_session_id() {
        let val = serde_json::json!({"session_id": "sess-abc-123", "ok": true});
        assert_eq!(
            parse_session_response(&val),
            Some("sess-abc-123".to_string())
        );
    }

    #[test]
    fn parse_session_response_extracts_nested_id() {
        let val = serde_json::json!({"ok": true, "session": {"id": "session-123", "status": "active"}});
        assert_eq!(parse_session_response(&val), Some("session-123".to_string()));
    }

    #[test]
    fn parse_session_response_returns_none_on_missing_key() {
        let val = serde_json::json!({"ok": true});
        assert_eq!(parse_session_response(&val), None);
    }

    // --- existing parse_message_response tests ---

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
