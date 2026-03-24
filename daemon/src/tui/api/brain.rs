// Brain Canvas fetch and parse — /api/brain

use reqwest::Client;
use serde_json::Value;

use crate::tui::{BrainNode, KpiData};

/// Parse /api/brain JSON response into (Vec<BrainNode>, KpiData).
/// Called by fetch_brain and directly by unit tests.
pub fn parse_brain_response(v: &Value) -> (Vec<BrainNode>, KpiData) {
    let mut nodes: Vec<BrainNode> = Vec::new();

    // Sessions -> kind="session"
    if let Some(sessions) = v.get("sessions").and_then(Value::as_array) {
        for s in sessions {
            let id = s
                .get("agent_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let label = s
                .get("description")
                .and_then(Value::as_str)
                .or_else(|| s.get("type").and_then(Value::as_str))
                .unwrap_or_default()
                .to_string();
            let status = s
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            nodes.push(BrainNode {
                id,
                label,
                kind: "session".to_string(),
                parent_id: None,
                status,
            });
        }
    }

    // Agents -> kind="agent"
    let agents_arr = v
        .get("agents")
        .or_else(|| v.get("running"))
        .and_then(Value::as_array);
    if let Some(agents) = agents_arr {
        for a in agents {
            let id = a
                .get("agent_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let label = a
                .get("description")
                .and_then(Value::as_str)
                .or_else(|| a.get("type").and_then(Value::as_str))
                .unwrap_or_default()
                .to_string();
            let status = a
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            nodes.push(BrainNode {
                id,
                label,
                kind: "agent".to_string(),
                parent_id: None,
                status,
            });
        }
    }

    // Extract token summary for KpiData if present
    let kpi = if let Some(summary) = v.get("today_tokens_summary") {
        let daily_tokens = summary
            .get("total_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let daily_cost = summary
            .get("total_cost")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        KpiData {
            daily_tokens,
            daily_cost,
            ..KpiData::default()
        }
    } else {
        KpiData::default()
    };

    (nodes, kpi)
}

/// GET {api_url}/api/brain -> (Vec<BrainNode>, KpiData)
pub async fn fetch_brain(client: &Client, api_url: &str) -> (Vec<BrainNode>, KpiData) {
    let url = format!("{api_url}/api/brain");
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(v) => parse_brain_response(&v),
            Err(_) => (Vec::new(), KpiData::default()),
        },
        Err(_) => (Vec::new(), KpiData::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_brain_response_maps_sessions_and_agents() {
        let json = serde_json::json!({
            "sessions": [{"agent_id":"task-executor","type":"executor",
                "description":"Running T2-03","status":"active"}],
            "agents": [{"agent_id":"thor","type":"validator",
                "description":"Wave validator","status":"idle"}],
            "today_tokens_summary": {"total_tokens":42000,"total_cost":1.25}
        });
        let (nodes, kpi) = parse_brain_response(&json);
        let session = nodes.iter().find(|n| n.kind == "session").expect("session");
        assert_eq!(session.id, "task-executor");
        assert_eq!(session.status, "active");
        let agent = nodes.iter().find(|n| n.kind == "agent").expect("agent");
        assert_eq!(agent.id, "thor");
        assert_eq!(kpi.daily_tokens, 42000);
        assert!((kpi.daily_cost - 1.25).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_brain_response_handles_empty() {
        let json = serde_json::json!({});
        let (nodes, kpi) = parse_brain_response(&json);
        assert!(nodes.is_empty());
        assert_eq!(kpi.daily_tokens, 0);
    }
}
