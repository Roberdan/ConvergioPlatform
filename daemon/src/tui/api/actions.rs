// Mesh and agent action API calls — POST to daemon endpoints.
// Returns a human-readable result string for display in the popup.

use reqwest::Client;
use serde_json::{json, Value};

/// POST {api_url}/api/mesh/exec with provision command for a specific node.
/// Returns stdout or a descriptive error message.
pub async fn mesh_provision(client: &Client, api_url: &str, node: &str) -> String {
    let url = format!("{api_url}/api/mesh/exec");
    let body = json!({"node": node, "command": "provision"});
    post_and_extract(client, &url, body).await
}

/// POST {api_url}/api/mesh/exec with heartbeat command (no target node).
/// Returns stdout or a descriptive error message.
pub async fn mesh_heartbeat(client: &Client, api_url: &str) -> String {
    let url = format!("{api_url}/api/mesh/exec");
    let body = json!({"command": "heartbeat"});
    post_and_extract(client, &url, body).await
}

/// POST {api_url}/api/ipc/agents/unregister to stop a named agent.
/// Returns confirmation or error message.
pub async fn stop_agent(client: &Client, api_url: &str, name: &str) -> String {
    let url = format!("{api_url}/api/ipc/agents/unregister");
    let body = json!({"name": name});
    post_and_extract(client, &url, body).await
}

/// Helper: POST JSON body, extract stdout/message/ok from response.
async fn post_and_extract(client: &Client, url: &str, body: Value) -> String {
    match client.post(url).json(&body).send().await {
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<Value>().await {
                Ok(v) => {
                    // Try stdout first (mesh/exec), then message (unregister), then ok.
                    if let Some(stdout) = v.get("stdout").and_then(Value::as_str) {
                        if !stdout.is_empty() {
                            return stdout.to_string();
                        }
                    }
                    if let Some(msg) = v.get("message").and_then(Value::as_str) {
                        return msg.to_string();
                    }
                    if let Some(ok) = v.get("ok").and_then(Value::as_bool) {
                        return if ok { "OK".to_string() } else { "Failed".to_string() };
                    }
                    if status.is_success() {
                        "Done".to_string()
                    } else {
                        format!("Error: HTTP {}", status.as_u16())
                    }
                }
                Err(_) => {
                    if status.is_success() {
                        "Done".to_string()
                    } else {
                        format!("Error: HTTP {}", status.as_u16())
                    }
                }
            }
        }
        Err(e) => format!("Request failed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Test the response parsing logic via a local helper that mirrors post_and_extract behavior.
    fn parse_mock_response(v: Value, status_ok: bool) -> String {
        if let Some(stdout) = v.get("stdout").and_then(Value::as_str) {
            if !stdout.is_empty() {
                return stdout.to_string();
            }
        }
        if let Some(msg) = v.get("message").and_then(Value::as_str) {
            return msg.to_string();
        }
        if let Some(ok) = v.get("ok").and_then(Value::as_bool) {
            return if ok { "OK".to_string() } else { "Failed".to_string() };
        }
        if status_ok {
            "Done".to_string()
        } else {
            "Error: HTTP 500".to_string()
        }
    }

    #[test]
    fn parse_stdout_from_mesh_exec_response() {
        let v = json!({"stdout": "provision complete", "exit_code": 0});
        assert_eq!(parse_mock_response(v, true), "provision complete");
    }

    #[test]
    fn parse_message_from_unregister_response() {
        let v = json!({"ok": true, "message": "agent stopped"});
        // message takes priority after stdout (which is absent here)
        assert_eq!(parse_mock_response(v, true), "agent stopped");
    }

    #[test]
    fn parse_ok_true_returns_ok() {
        let v = json!({"ok": true});
        assert_eq!(parse_mock_response(v, true), "OK");
    }

    #[test]
    fn parse_ok_false_returns_failed() {
        let v = json!({"ok": false});
        assert_eq!(parse_mock_response(v, true), "Failed");
    }

    #[test]
    fn parse_empty_stdout_falls_through_to_message() {
        let v = json!({"stdout": "", "message": "heartbeat sent"});
        assert_eq!(parse_mock_response(v, true), "heartbeat sent");
    }

    #[test]
    fn parse_empty_body_success_returns_done() {
        let v = json!({});
        assert_eq!(parse_mock_response(v, true), "Done");
    }

    #[test]
    fn parse_empty_body_error_returns_error() {
        let v = json!({});
        assert_eq!(parse_mock_response(v, false), "Error: HTTP 500");
    }
}
