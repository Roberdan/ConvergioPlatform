// Plan and task drill-down fetch functions → PopupContent.

use reqwest::Client;
use serde_json::Value;

use crate::tui::views::popup::{PopupContent, PopupSection};

pub(super) fn error_popup(msg: &str) -> PopupContent {
    PopupContent {
        title: "Error".to_string(),
        sections: vec![PopupSection {
            label: "Error".to_string(),
            lines: vec![msg.to_string()],
        }],
        actions: vec![],
    }
}

pub(super) fn str_field<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("—")
}

pub(super) fn i64_field(v: &Value, key: &str) -> i64 {
    v.get(key).and_then(Value::as_i64).unwrap_or(0)
}

/// Parse /api/plan-db/json/{plan_id} into a plan PopupContent.
/// Separated for unit testing without a live server.
pub fn parse_plan_detail(v: &Value) -> PopupContent {
    let plan = v.get("plan").unwrap_or(v);
    let name = str_field(plan, "name");
    let status = str_field(plan, "status");
    let id = i64_field(plan, "id");

    let info_section = PopupSection {
        label: "Plan Info".to_string(),
        lines: vec![
            format!("ID:     {id}"),
            format!("Name:   {name}"),
            format!("Status: {status}"),
        ],
    };

    let waves = v
        .get("waves")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|w| {
                    format!(
                        "{} — {}",
                        w.get("wave_id").and_then(Value::as_str).unwrap_or("?"),
                        w.get("status").and_then(Value::as_str).unwrap_or("?"),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let waves_section = PopupSection {
        label: "Waves".to_string(),
        lines: if waves.is_empty() { vec!["No waves".to_string()] } else { waves },
    };

    let tasks_done = i64_field(plan, "tasks_done");
    let tasks_total = i64_field(plan, "tasks_total");
    let tasks_section = PopupSection {
        label: "Tasks".to_string(),
        lines: vec![format!("{tasks_done}/{tasks_total} done")],
    };

    PopupContent {
        title: format!("Plan: {name}"),
        sections: vec![info_section, waves_section, tasks_section],
        actions: vec![],
    }
}

/// GET {api_url}/api/plan-db/json/{plan_id} → PopupContent
pub async fn fetch_plan_detail(client: &Client, api_url: &str, plan_id: i64) -> PopupContent {
    let url = format!("{api_url}/api/plan-db/json/{plan_id}");
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(v) => parse_plan_detail(&v),
            Err(e) => error_popup(&format!("parse error: {e}")),
        },
        Err(e) => error_popup(&format!("fetch error: {e}")),
    }
}

/// GET {api_url}/api/plan/{plan_id} → find task at index → PopupContent
pub async fn fetch_task_detail(
    client: &Client,
    api_url: &str,
    plan_id: i64,
    task_idx: usize,
) -> PopupContent {
    let url = format!("{api_url}/api/plan/{plan_id}");
    let v = match client.get(&url).send().await {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(v) => v,
            Err(e) => return error_popup(&format!("parse error: {e}")),
        },
        Err(e) => return error_popup(&format!("fetch error: {e}")),
    };

    let tasks = v
        .get("tasks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let task = match tasks.get(task_idx) {
        Some(t) => t.clone(),
        None => return error_popup(&format!("task index {task_idx} not found")),
    };

    let task_section = PopupSection {
        label: "Task".to_string(),
        lines: vec![
            format!("ID:     {}", str_field(&task, "task_id")),
            format!("Title:  {}", str_field(&task, "title")),
            format!("Status: {}", str_field(&task, "status")),
            format!("Agent:  {}", str_field(&task, "executor_agent")),
        ],
    };

    let mut sections = vec![task_section];
    let criteria = task
        .get("test_criteria")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !criteria.is_empty() {
        sections.push(PopupSection {
            label: "Test Criteria".to_string(),
            lines: criteria.lines().map(|l| l.to_string()).collect(),
        });
    }

    PopupContent {
        title: format!("Task: {}", str_field(&task, "task_id")),
        sections,
        actions: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plan_detail_maps_all_sections() {
        let json = serde_json::json!({
            "plan": {
                "id": 709,
                "name": "Plan H — TUI Interactive",
                "status": "doing",
                "tasks_done": 3,
                "tasks_total": 12
            },
            "waves": [
                {"wave_id": "W1", "status": "in_progress"},
                {"wave_id": "W2", "status": "pending"}
            ]
        });

        let popup = parse_plan_detail(&json);
        assert_eq!(popup.title, "Plan: Plan H — TUI Interactive");
        assert_eq!(popup.sections.len(), 3);
        assert_eq!(popup.sections[0].label, "Plan Info");
        assert!(popup.sections[0].lines.iter().any(|l| l.contains("709")));
        assert!(popup.sections[0].lines.iter().any(|l| l.contains("doing")));
        assert_eq!(popup.sections[1].label, "Waves");
        assert!(popup.sections[1].lines.iter().any(|l| l.contains("W1")));
        assert!(popup.sections[1].lines.iter().any(|l| l.contains("W2")));
        assert_eq!(popup.sections[2].label, "Tasks");
        assert!(popup.sections[2].lines[0].contains("3/12"));
        assert!(popup.actions.is_empty());
    }

    #[test]
    fn parse_plan_detail_handles_missing_waves() {
        let json = serde_json::json!({
            "plan": {"id": 1, "name": "Solo", "status": "todo", "tasks_done": 0, "tasks_total": 0}
        });
        let popup = parse_plan_detail(&json);
        assert_eq!(popup.sections[1].lines, vec!["No waves"]);
    }
}
