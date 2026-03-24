// Event, workspace, and deliverable drill-down fetch functions → PopupContent.

use reqwest::Client;
use serde_json::Value;

use crate::tui::views::popup::{PopupContent, PopupSection};
use crate::tui::WorkspaceEvent;

use super::detail_plan::{error_popup, i64_field, str_field};

/// Format a WorkspaceEvent as a PopupContent without any API call.
pub fn format_event_detail(event: &WorkspaceEvent) -> PopupContent {
    let event_section = PopupSection {
        label: "Event".to_string(),
        lines: vec![
            format!("ID:        {}", event.id),
            format!("Workspace: {}", event.workspace_id),
            format!("Agent:     {}", event.agent),
            format!("Action:    {}", event.action),
            format!("Time:      {}", event.created_at),
        ],
    };

    let detail_section = PopupSection {
        label: "Detail".to_string(),
        lines: vec![
            format!("File:     {}", event.file_path.as_deref().unwrap_or("—")),
            format!("Detail:   {}", event.detail.as_deref().unwrap_or("—")),
            // Metadata placeholder — extend when WorkspaceEvent gains a metadata field.
            "Metadata: —".to_string(),
        ],
    };

    PopupContent {
        title: format!("Event #{}", event.id),
        sections: vec![event_section, detail_section],
        actions: vec![],
    }
}

/// GET {api_url}/api/workspace/status/{workspace_id} → PopupContent
pub async fn fetch_workspace_detail(
    client: &Client,
    api_url: &str,
    workspace_id: &str,
) -> PopupContent {
    let url = format!("{api_url}/api/workspace/status/{workspace_id}");
    let v = match client.get(&url).send().await {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(v) => v,
            Err(e) => return error_popup(&format!("parse error: {e}")),
        },
        Err(e) => return error_popup(&format!("fetch error: {e}")),
    };

    let ws_section = PopupSection {
        label: "Workspace".to_string(),
        lines: vec![
            format!("ID:     {}", str_field(&v, "workspace_id")),
            format!("Path:   {}", str_field(&v, "path")),
            format!("Branch: {}", str_field(&v, "branch")),
            format!("Status: {}", str_field(&v, "status")),
        ],
    };

    let recent_events = v
        .get("recent_events")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .take(5)
                .map(|e| {
                    format!(
                        "{} {} {}",
                        e.get("action").and_then(Value::as_str).unwrap_or("?"),
                        e.get("agent").and_then(Value::as_str).unwrap_or("?"),
                        e.get("created_at").and_then(Value::as_str).unwrap_or("?"),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let events_section = PopupSection {
        label: "Recent Events".to_string(),
        lines: if recent_events.is_empty() {
            vec!["No events".to_string()]
        } else {
            recent_events
        },
    };

    PopupContent {
        title: format!("Workspace: {workspace_id}"),
        sections: vec![ws_section, events_section],
        actions: vec![('g', "Quality Gate".to_string())],
    }
}

/// GET {api_url}/api/deliverables/{id} → PopupContent
pub async fn fetch_deliverable_detail(
    client: &Client,
    api_url: &str,
    id: i64,
) -> PopupContent {
    let url = format!("{api_url}/api/deliverables/{id}");
    let v = match client.get(&url).send().await {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(v) => v,
            Err(e) => return error_popup(&format!("parse error: {e}")),
        },
        Err(e) => return error_popup(&format!("fetch error: {e}")),
    };

    let name = str_field(&v, "name").to_string();
    let deliv_section = PopupSection {
        label: "Deliverable".to_string(),
        lines: vec![
            format!("Name:    {name}"),
            format!("Type:    {}", str_field(&v, "output_type")),
            format!("Status:  {}", str_field(&v, "status")),
            format!("Version: {}", i64_field(&v, "version")),
            format!("Project: {}", str_field(&v, "project_id")),
        ],
    };

    let mut sections = vec![deliv_section];
    if let Some(meta) = v.get("metadata") {
        if !meta.is_null() {
            let meta_str = serde_json::to_string_pretty(meta)
                .unwrap_or_else(|_| "—".to_string());
            sections.push(PopupSection {
                label: "Metadata".to_string(),
                lines: meta_str.lines().map(|l| l.to_string()).collect(),
            });
        }
    }

    PopupContent {
        title: format!("Deliverable: {name}"),
        sections,
        actions: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_event_detail_maps_all_fields() {
        let event = WorkspaceEvent {
            id: 42,
            workspace_id: "ws-abc".to_string(),
            agent: "executor".to_string(),
            action: "file_write".to_string(),
            file_path: Some("src/lib.rs".to_string()),
            detail: Some("added module".to_string()),
            created_at: "2026-03-24T10:00:00Z".to_string(),
        };

        let popup = format_event_detail(&event);
        assert_eq!(popup.title, "Event #42");
        assert_eq!(popup.sections[0].label, "Event");
        assert!(popup.sections[0].lines.iter().any(|l| l.contains("ws-abc")));
        assert!(popup.sections[0].lines.iter().any(|l| l.contains("executor")));
        assert!(popup.sections[0].lines.iter().any(|l| l.contains("file_write")));
        assert_eq!(popup.sections[1].label, "Detail");
        assert!(popup.sections[1].lines.iter().any(|l| l.contains("src/lib.rs")));
        assert!(popup.sections[1].lines.iter().any(|l| l.contains("added module")));
        assert!(popup.actions.is_empty());
    }

    #[test]
    fn format_event_detail_handles_missing_optionals() {
        let event = WorkspaceEvent {
            id: 1,
            workspace_id: "ws-1".to_string(),
            agent: "thor".to_string(),
            action: "git_commit".to_string(),
            file_path: None,
            detail: None,
            created_at: "2026-03-24T00:00:00Z".to_string(),
        };
        let popup = format_event_detail(&event);
        assert!(popup.sections[1].lines.iter().any(|l| l.contains("—")));
    }
}
