// Chat API — run claude CLI locally for chat responses. No API keys.
use reqwest::Client;
use serde_json::Value;

/// POST /api/chat/session — creates a new chat session, returns session_id.
pub async fn create_session(client: &Client, api_url: &str) -> Option<String> {
    let url = format!("{api_url}/api/chat/session");
    let resp = client.post(&url).json(&serde_json::json!({})).send().await.ok()?;
    let val: Value = resp.json().await.ok()?;
    val.get("session_id").and_then(Value::as_str).map(String::from)
        .or_else(|| val.get("session").and_then(|s| s.get("id")).and_then(Value::as_str).map(String::from))
}

/// Run `claude --print` with Ali (Chief of Staff) persona locally.
/// Uses the logged-in Claude session — no API key needed.
pub async fn send_message(
    _client: &Client, _api_url: &str, _session_id: &str, content: &str,
) -> Option<String> {
    use tokio::process::Command;
    let ali_prompt = "You are Ali, Chief of Staff of the Convergio Platform. \
        You orchestrate all agents, know every plan, task, and mesh node. \
        Respond concisely in the user's language. You have full context on \
        ConvergioPlatform: daemon (Rust), dashboard, evolution engine, mesh network. \
        Be helpful, direct, and actionable.";
    let output = Command::new("claude")
        .args(["--print", "--append-system-prompt", ali_prompt, content])
        .output()
        .await
        .ok()?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if stdout.is_empty() { None } else { Some(stdout) }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Some(format!("(claude error: {})", if stderr.is_empty() { "unknown" } else { &stderr }))
    }
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
