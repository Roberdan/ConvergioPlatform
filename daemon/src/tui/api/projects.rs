// Project list API fetch — /api/projects

use reqwest::Client;
use serde_json::Value;

use crate::tui::data::ProjectInfo;

/// Parse /api/projects JSON response into Vec<ProjectInfo>.
pub fn parse_projects_response(val: &Value) -> Vec<ProjectInfo> {
    val.get("projects")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|item| ProjectInfo {
                    id: item.get("id").and_then(Value::as_i64).unwrap_or(0),
                    name: item
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    path: item
                        .get("path")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// GET {api_url}/api/projects -> Vec<ProjectInfo>
/// Returns empty vec on any error.
pub async fn fetch_projects(
    client: &Client,
    api_url: &str,
) -> Vec<ProjectInfo> {
    let url = format!("{api_url}/api/projects");
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(v) => parse_projects_response(&v),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_projects_response_maps_fields() {
        let json = serde_json::json!({
            "ok": true,
            "projects": [
                {"id": 1, "name": "ConvergioPlatform", "path": "/repos/convergio"}
            ]
        });
        let projects = parse_projects_response(&json);
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, 1);
        assert_eq!(projects[0].name, "ConvergioPlatform");
        assert_eq!(projects[0].path, "/repos/convergio");
    }

    #[test]
    fn parse_projects_response_handles_empty() {
        let json = serde_json::json!({"ok": true, "projects": []});
        assert!(parse_projects_response(&json).is_empty());
    }

    #[test]
    fn parse_projects_response_handles_missing_key() {
        let json = serde_json::json!({"ok": true});
        assert!(parse_projects_response(&json).is_empty());
    }
}
