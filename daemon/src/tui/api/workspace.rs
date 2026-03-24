// Workspace API fetch — /api/workspace/list

use reqwest::Client;
use serde_json::Value;

use crate::tui::WorkspaceInfo;

/// Parse /api/workspace/list JSON response into Vec<WorkspaceInfo>.
/// Separated from fetch to allow unit testing without a live server.
pub fn parse_workspaces_response(val: &Value) -> Vec<WorkspaceInfo> {
    val.get("workspaces")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|item| WorkspaceInfo {
                    workspace_id: item
                        .get("workspace_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    path: item
                        .get("path")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    branch: item
                        .get("branch")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    plan_id: item.get("plan_id").and_then(Value::as_i64),
                    status: item
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
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

/// GET {api_url}/api/workspace/list -> Vec<WorkspaceInfo>
/// Returns empty vec on any error (network failure, parse error, server error).
pub async fn fetch_workspaces(client: &Client, api_url: &str) -> Vec<WorkspaceInfo> {
    let url = format!("{api_url}/api/workspace/list");
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(v) => parse_workspaces_response(&v),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_workspaces_response_maps_all_fields() {
        let json = serde_json::json!({
            "ok": true,
            "workspaces": [
                {
                    "workspace_id": "ws-123",
                    "path": "/tmp/ws",
                    "branch": "plan-708",
                    "plan_id": 708,
                    "status": "active",
                    "created_at": "2026-03-23T00:00:00Z"
                }
            ]
        });

        let workspaces = parse_workspaces_response(&json);
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].workspace_id, "ws-123");
        assert_eq!(workspaces[0].path, "/tmp/ws");
        assert_eq!(workspaces[0].branch, "plan-708");
        assert_eq!(workspaces[0].plan_id, Some(708));
        assert_eq!(workspaces[0].status, "active");
        assert_eq!(workspaces[0].created_at, "2026-03-23T00:00:00Z");
    }

    #[test]
    fn parse_workspaces_response_handles_empty_array() {
        let json = serde_json::json!({"ok": true, "workspaces": []});
        let workspaces = parse_workspaces_response(&json);
        assert!(workspaces.is_empty());
    }

    #[test]
    fn parse_workspaces_response_handles_missing_key() {
        let json = serde_json::json!({"ok": true});
        let workspaces = parse_workspaces_response(&json);
        assert!(workspaces.is_empty());
    }

    #[test]
    fn parse_workspaces_response_handles_null_plan_id() {
        let json = serde_json::json!({
            "ok": true,
            "workspaces": [
                {
                    "workspace_id": "ws-456",
                    "path": "/tmp/ws2",
                    "branch": "main",
                    "plan_id": null,
                    "status": "merged",
                    "created_at": "2026-03-22T00:00:00Z"
                }
            ]
        });
        let workspaces = parse_workspaces_response(&json);
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].plan_id, None);
        assert_eq!(workspaces[0].status, "merged");
    }

    #[test]
    fn parse_workspaces_response_handles_multiple_workspaces() {
        let json = serde_json::json!({
            "ok": true,
            "workspaces": [
                {
                    "workspace_id": "ws-1",
                    "path": "/tmp/ws1",
                    "branch": "plan-100",
                    "plan_id": 100,
                    "status": "active",
                    "created_at": "2026-03-20T00:00:00Z"
                },
                {
                    "workspace_id": "ws-2",
                    "path": "/tmp/ws2",
                    "branch": "plan-200",
                    "plan_id": 200,
                    "status": "deleted",
                    "created_at": "2026-03-21T00:00:00Z"
                }
            ]
        });
        let workspaces = parse_workspaces_response(&json);
        assert_eq!(workspaces.len(), 2);
        assert_eq!(workspaces[0].workspace_id, "ws-1");
        assert_eq!(workspaces[1].workspace_id, "ws-2");
        assert_eq!(workspaces[1].status, "deleted");
    }
}
