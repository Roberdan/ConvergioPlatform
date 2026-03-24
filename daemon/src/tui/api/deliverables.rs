// Deliverables API fetch — /api/deliverables

use reqwest::Client;
use serde_json::Value;

use crate::tui::DeliverableInfo;

/// Parse /api/deliverables JSON response into Vec<DeliverableInfo>.
/// Response is a top-level array (not wrapped in an object key).
/// Separated from fetch to allow unit testing without a live server.
pub fn parse_deliverables_response(val: &Value) -> Vec<DeliverableInfo> {
    val.as_array()
        .map(|arr| {
            arr.iter()
                .map(|item| DeliverableInfo {
                    id: item.get("id").and_then(Value::as_i64).unwrap_or(0),
                    name: item
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    output_type: item
                        .get("output_type")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    status: item
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    version: item.get("version").and_then(Value::as_i64).unwrap_or(1),
                    project_id: item
                        .get("project_id")
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

/// GET {api_url}/api/deliverables -> Vec<DeliverableInfo>
/// Returns empty vec on any error (network failure, parse error, server error).
pub async fn fetch_deliverables(client: &Client, api_url: &str) -> Vec<DeliverableInfo> {
    let url = format!("{api_url}/api/deliverables");
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(v) => parse_deliverables_response(&v),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_deliverables_response_maps_all_fields() {
        let json = serde_json::json!([
            {
                "id": 1,
                "name": "Report",
                "output_type": "doc",
                "status": "approved",
                "version": 1,
                "project_id": "convergio",
                "created_at": "2026-03-23T00:00:00Z"
            }
        ]);

        let deliverables = parse_deliverables_response(&json);
        assert_eq!(deliverables.len(), 1);
        assert_eq!(deliverables[0].id, 1);
        assert_eq!(deliverables[0].name, "Report");
        assert_eq!(deliverables[0].output_type, "doc");
        assert_eq!(deliverables[0].status, "approved");
        assert_eq!(deliverables[0].version, 1);
        assert_eq!(deliverables[0].project_id, "convergio");
        assert_eq!(deliverables[0].created_at, "2026-03-23T00:00:00Z");
    }

    #[test]
    fn parse_deliverables_response_handles_empty_array() {
        let json = serde_json::json!([]);
        let deliverables = parse_deliverables_response(&json);
        assert!(deliverables.is_empty());
    }

    #[test]
    fn parse_deliverables_response_handles_non_array() {
        let json = serde_json::json!({"error": "not found"});
        let deliverables = parse_deliverables_response(&json);
        assert!(deliverables.is_empty());
    }

    #[test]
    fn parse_deliverables_response_handles_multiple_statuses() {
        let json = serde_json::json!([
            {
                "id": 1,
                "name": "Alpha",
                "output_type": "pdf",
                "status": "pending",
                "version": 1,
                "project_id": "proj-a",
                "created_at": "2026-03-20T00:00:00Z"
            },
            {
                "id": 2,
                "name": "Beta",
                "output_type": "doc",
                "status": "rejected",
                "version": 2,
                "project_id": "proj-b",
                "created_at": "2026-03-21T00:00:00Z"
            },
            {
                "id": 3,
                "name": "Gamma",
                "output_type": "xlsx",
                "status": "approved",
                "version": 3,
                "project_id": "proj-c",
                "created_at": "2026-03-22T00:00:00Z"
            }
        ]);

        let deliverables = parse_deliverables_response(&json);
        assert_eq!(deliverables.len(), 3);
        assert_eq!(deliverables[0].status, "pending");
        assert_eq!(deliverables[1].status, "rejected");
        assert_eq!(deliverables[2].status, "approved");
        assert_eq!(deliverables[2].version, 3);
    }

    #[test]
    fn parse_deliverables_response_defaults_missing_version() {
        let json = serde_json::json!([
            {
                "id": 5,
                "name": "NoVersion",
                "output_type": "doc",
                "status": "pending",
                "project_id": "proj-x",
                "created_at": "2026-03-23T00:00:00Z"
            }
        ]);
        let deliverables = parse_deliverables_response(&json);
        assert_eq!(deliverables[0].version, 1);
    }
}
