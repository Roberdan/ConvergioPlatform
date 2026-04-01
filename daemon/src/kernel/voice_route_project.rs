// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Handlers for CreateProject, AskOrg, and CreateOrgFrom voice intents.
// Uses org factory + provisioner for full org lifecycle via daemon HTTP API.

use crate::org::factory::{design_org_from_mission, design_org_from_repo};
use crate::org::orgchart::render_orgchart_compact;
use crate::org::provisioner::provision_org;
use crate::org::repo_scanner::scan_repo;
use serde_json::Value;
use std::path::Path;

const DEFAULT_BUDGET: f64 = 50.0;

/// Build a reqwest blocking client with 10s timeout.
fn http_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

/// Create a full project via org factory: design blueprint, provision, render.
/// Returns Italian Telegram-friendly response string.
pub(crate) fn route_create_project(name: &str, mission: &str, base_url: &str) -> String {
    let mission_text = if mission.is_empty() {
        format!("Progetto {name}")
    } else {
        mission.to_string()
    };
    let blueprint = design_org_from_mission(name, &mission_text, DEFAULT_BUDGET);
    match provision_org(&blueprint, base_url) {
        Ok(res) => {
            let chart = render_orgchart_compact(&blueprint);
            format!(
                "Progetto '{name}' creato! Org: {}, Piano: {} \
                 ({} agenti, {} task).\n\n{chart}",
                res.org_id, res.plan_id, res.agents_created, res.tasks_created,
            )
        }
        Err(e) => format!("Errore creazione org: {e}"),
    }
}

/// Scan a repo, design an org from its profile, provision, and render.
/// Returns Italian Telegram-friendly response string.
pub(crate) fn route_create_org_from(path: &str, base_url: &str) -> String {
    let profile = match scan_repo(Path::new(path)) {
        Ok(p) => p,
        Err(e) => return format!("Errore scansione repo: {e}"),
    };
    let blueprint = design_org_from_repo(&profile, None, DEFAULT_BUDGET);
    match provision_org(&blueprint, base_url) {
        Ok(res) => {
            let chart = render_orgchart_compact(&blueprint);
            format!(
                "Org '{}' creata da repo! Piano: {} ({} agenti, {} task).\n\n{chart}",
                blueprint.name, res.plan_id, res.agents_created, res.tasks_created,
            )
        }
        Err(e) => format!("Errore provisioning org: {e}"),
    }
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
    use serde_json::json;

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
