// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Handlers for CreateProject and AskOrg voice intents.
// Creates org + bootstrap plan + initial tasks via daemon HTTP API.

use serde_json::{json, Value};
use std::time::Duration;
use tracing::warn;

/// Build a reqwest blocking client with 10s timeout for project creation calls.
fn http_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

/// Create a full project: org + bootstrap plan + 3 initial tasks.
/// Returns Italian Telegram-friendly response string.
pub(crate) fn route_create_project(name: &str, mission: &str, base_url: &str) -> String {
    let client = http_client();
    let slug = name.to_lowercase().replace(' ', "-");
    let mission_text = if mission.is_empty() {
        format!("Progetto {name}")
    } else {
        mission.to_string()
    };

    // Step 1: Create org
    let org_id = match create_org(&client, base_url, &slug, &mission_text, name) {
        Ok(id) => id,
        Err(e) => return format!("Errore creazione org: {e}"),
    };

    // Step 2: Create bootstrap plan
    let plan_id = match create_plan(&client, base_url, name, &mission_text) {
        Ok(id) => id,
        Err(e) => return format!("Org '{org_id}' creata, ma errore piano: {e}"),
    };

    // Step 3: Create initial tasks
    let tasks = [
        format!("Define scope and objectives for {name}"),
        format!("Create execution plan with milestones for {name}"),
        format!("Begin execution and report progress for {name}"),
    ];
    let mut created = 0u32;
    for task_title in &tasks {
        match create_task(&client, base_url, plan_id, task_title) {
            Ok(_) => created += 1,
            Err(e) => warn!("voice_route_project: task creation failed: {e}"),
        }
    }

    format!(
        "Progetto '{name}' creato! Org: {org_id}, Piano: {plan_id} con {created} task. \
         Ali lo sta eseguendo."
    )
}

fn create_org(
    client: &reqwest::blocking::Client,
    base_url: &str,
    slug: &str,
    mission: &str,
    name: &str,
) -> Result<String, String> {
    let body = json!({
        "id": slug,
        "mission": mission,
        "objectives": format!("Bootstrap project {name}"),
        "ceo_agent": format!("{slug}-ceo"),
        "budget": 50.0
    });
    let resp = client
        .post(format!("{base_url}/api/orgs"))
        .json(&body)
        .send()
        .map_err(|e| e.to_string())?;
    let v: Value = resp.json().map_err(|e| e.to_string())?;
    v.get("org_id")
        .and_then(|id| id.as_str())
        .map(String::from)
        .ok_or_else(|| "missing org_id in response".to_string())
}

fn create_plan(
    client: &reqwest::blocking::Client,
    base_url: &str,
    name: &str,
    mission: &str,
) -> Result<u64, String> {
    let body = json!({
        "title": format!("Bootstrap: {name}"),
        "description": mission
    });
    let resp = client
        .post(format!("{base_url}/api/plan-db/create"))
        .json(&body)
        .send()
        .map_err(|e| e.to_string())?;
    let v: Value = resp.json().map_err(|e| e.to_string())?;
    v.get("plan_id")
        .and_then(|id| id.as_u64())
        .ok_or_else(|| "missing plan_id in response".to_string())
}

fn create_task(
    client: &reqwest::blocking::Client,
    base_url: &str,
    plan_id: u64,
    title: &str,
) -> Result<u64, String> {
    let body = json!({
        "plan_id": plan_id,
        "title": title,
        "status": "pending"
    });
    let resp = client
        .post(format!("{base_url}/api/plan-db/task/create"))
        .json(&body)
        .send()
        .map_err(|e| e.to_string())?;
    let v: Value = resp.json().map_err(|e| e.to_string())?;
    v.get("task_id")
        .and_then(|id| id.as_u64())
        .ok_or_else(|| "missing task_id in response".to_string())
}

/// Query an org's status by name. Tries exact match first, then fuzzy.
/// Returns Italian Telegram-friendly summary.
pub(crate) fn route_ask_org(name: &str, base_url: &str) -> String {
    let client = http_client();
    let slug = name.to_lowercase().replace(' ', "-");

    // Try direct org fetch
    match client
        .get(format!("{base_url}/api/orgs/{slug}"))
        .send()
    {
        Ok(resp) => match resp.json::<Value>() {
            Ok(v) if v.get("ok").and_then(|o| o.as_bool()) == Some(true) => {
                return format_org_summary(&v, name);
            }
            _ => {}
        },
        Err(_) => {}
    }

    // Fallback: list orgs and find by name fragment
    match client.get(format!("{base_url}/api/orgs")).send() {
        Ok(resp) => match resp.json::<Value>() {
            Ok(v) => {
                let ids: Vec<String> = v["orgs"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|o| o["id"].as_str().map(String::from))
                    .collect();
                if let Some(org_id) =
                    crate::kernel::org_router::extract_org_reference(name, &ids)
                {
                    if let Ok(r2) = client
                        .get(format!("{base_url}/api/orgs/{org_id}"))
                        .send()
                    {
                        if let Ok(v2) = r2.json::<Value>() {
                            return format_org_summary(&v2, &org_id);
                        }
                    }
                }
                format!("Nessuna org trovata per '{name}'.")
            }
            Err(e) => format!("Errore parsing org list: {e}"),
        },
        Err(e) => format!("Errore contattando daemon: {e}"),
    }
}

fn format_org_summary(v: &Value, name: &str) -> String {
    let status = v["org"]["status"].as_str().unwrap_or("unknown");
    let mission = v["org"]["mission"].as_str().unwrap_or("-");
    let members = v["members"].as_array().map(|a| a.len()).unwrap_or(0);
    let budget = v["org"]["budget"].as_f64().unwrap_or(0.0);
    format!(
        "Org '{name}': stato={status}, missione=\"{mission}\", \
         {members} membri, budget={budget:.2} USD."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_org_summary_extracts_fields() {
        let v = json!({
            "ok": true,
            "org": {"status": "active", "mission": "ship fast", "budget": 100.0},
            "members": [{"agent": "ceo"}, {"agent": "dev"}],
            "services": []
        });
        let s = format_org_summary(&v, "alpha");
        assert!(s.contains("alpha"));
        assert!(s.contains("active"));
        assert!(s.contains("ship fast"));
        assert!(s.contains("2 membri"));
        assert!(s.contains("100.00"));
    }

    #[test]
    fn format_org_summary_handles_missing_fields() {
        let v = json!({"ok": true, "org": {}, "members": []});
        let s = format_org_summary(&v, "beta");
        assert!(s.contains("beta"));
        assert!(s.contains("unknown"));
    }
}
