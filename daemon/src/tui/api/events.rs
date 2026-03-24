// Event Stream API fetch — /api/workspace/events

use reqwest::Client;
use serde_json::Value;

use crate::tui::WorkspaceEvent;

/// Parse /api/workspace/events JSON response into Vec<WorkspaceEvent>.
/// Separated from fetch to allow unit testing without a live server.
pub fn parse_events_response(val: &Value) -> Vec<WorkspaceEvent> {
    val.get("events")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|item| WorkspaceEvent {
                    id: item.get("id").and_then(Value::as_i64).unwrap_or(0),
                    workspace_id: item
                        .get("workspace_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    agent: item
                        .get("agent")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    action: item
                        .get("action")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    file_path: item
                        .get("file_path")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string()),
                    detail: item
                        .get("detail")
                        .and_then(Value::as_str)
                        .map(|s| s.to_string()),
                    created_at: item
                        .get("created_at")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// GET {api_url}/api/workspace/events?limit=50 -> Vec<WorkspaceEvent>
/// Returns empty vec on any error (network failure, parse error, server error).
pub async fn fetch_events(client: &Client, api_url: &str) -> Vec<WorkspaceEvent> {
    let url = format!("{api_url}/api/workspace/events?limit=50");
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(v) => parse_events_response(&v),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_events_response_maps_all_fields() {
        let json = serde_json::json!({
            "ok": true,
            "events": [
                {
                    "id": 1,
                    "workspace_id": "ws-1",
                    "agent": "executor",
                    "action": "file_write",
                    "file_path": "src/foo.rs",
                    "detail": null,
                    "created_at": "2026-03-23T10:44:15Z"
                },
                {
                    "id": 2,
                    "workspace_id": "ws-1",
                    "agent": "thor",
                    "action": "git_commit",
                    "file_path": null,
                    "detail": "feat: add events view",
                    "created_at": "2026-03-23T10:45:00Z"
                }
            ]
        });

        let events = parse_events_response(&json);
        assert_eq!(events.len(), 2);

        assert_eq!(events[0].id, 1);
        assert_eq!(events[0].workspace_id, "ws-1");
        assert_eq!(events[0].agent, "executor");
        assert_eq!(events[0].action, "file_write");
        assert_eq!(events[0].file_path, Some("src/foo.rs".to_string()));
        assert_eq!(events[0].detail, None);
        assert_eq!(events[0].created_at, "2026-03-23T10:44:15Z");

        assert_eq!(events[1].id, 2);
        assert_eq!(events[1].action, "git_commit");
        assert_eq!(events[1].detail, Some("feat: add events view".to_string()));
    }

    #[test]
    fn parse_events_response_handles_empty_events_array() {
        let json = serde_json::json!({"ok": true, "events": []});
        let events = parse_events_response(&json);
        assert!(events.is_empty());
    }

    #[test]
    fn parse_events_response_handles_missing_events_key() {
        let json = serde_json::json!({"ok": true});
        let events = parse_events_response(&json);
        assert!(events.is_empty());
    }

    #[test]
    fn parse_events_response_handles_missing_optional_fields() {
        let json = serde_json::json!({
            "ok": true,
            "events": [
                {
                    "id": 5,
                    "workspace_id": "ws-2",
                    "agent": "planner",
                    "action": "pr_created",
                    "created_at": "2026-03-23T11:00:00Z"
                }
            ]
        });
        let events = parse_events_response(&json);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].file_path, None);
        assert_eq!(events[0].detail, None);
    }
}
