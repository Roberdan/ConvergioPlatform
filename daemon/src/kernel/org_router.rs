use serde_json::Value;

pub fn extract_org_reference(question: &str, org_ids: &[String]) -> Option<String> {
    let q = normalize(question);
    org_ids
        .iter()
        .map(|id| (id, normalize(id)))
        .find(|(_, normalized)| q.contains(normalized) || normalized.split('-').any(|p| !p.is_empty() && q.contains(p)))
        .map(|(id, _)| id.to_string())
}

pub fn summarize_digest_for_voice(org_id: &str, digest: &Value) -> String {
    let completed = digest["task_stats"]["completed"].as_i64().unwrap_or(0);
    let total = digest["task_stats"]["total"].as_i64().unwrap_or(0);
    let cost = digest["telemetry"]["cost"].as_f64().unwrap_or(0.0);
    format!("✅ {org_id}: {completed}/{total} task completati, costo {cost:.2} USD")
}

pub fn try_route_org_question(question: &str, daemon_url: &str) -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .ok()?;
    let orgs_payload = client
        .get(format!("{daemon_url}/api/orgs"))
        .send()
        .ok()?
        .json::<Value>()
        .ok()?;
    let org_ids: Vec<String> = orgs_payload["orgs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|o| o.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect();
    let org_id = extract_org_reference(question, &org_ids)?;
    let digest_payload = client
        .get(format!("{daemon_url}/api/orgs/{org_id}/digest"))
        .send()
        .ok()?
        .json::<Value>()
        .ok()?;
    let digest_content = digest_payload["digest"]["content"]
        .as_str()
        .and_then(|s| serde_json::from_str::<Value>(s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    Some(summarize_digest_for_voice(&org_id, &digest_content))
}

fn normalize(input: &str) -> String {
    input.to_lowercase().replace(['?', '!', ',', '.', ':'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_org_name_from_natural_language() {
        let orgs = vec![
            "fitness-project".to_string(),
            "legal-ops".to_string(),
            "platform".to_string(),
        ];
        let found = extract_org_reference("come sta il progetto fitness?", &orgs);
        assert_eq!(found.as_deref(), Some("fitness-project"));
    }

    #[test]
    fn routes_to_correct_org_digest_summary() {
        let digest = json!({
            "telemetry": { "cost": 12.5 },
            "task_stats": { "completed": 5, "total": 8 }
        });
        let summary = summarize_digest_for_voice("fitness-project", &digest);
        assert!(summary.contains("fitness-project"));
        assert!(summary.contains("5/8"));
        assert!(summary.contains("12.50"));
    }
}
