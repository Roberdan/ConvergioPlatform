// Node and agent drill-down fetch functions → PopupContent.

use reqwest::Client;
use serde_json::Value;

use crate::tui::views::popup::{PopupContent, PopupSection};

use super::detail_plan::{error_popup, str_field};

/// Parse /api/mesh JSON, find peer by name, return PopupContent.
pub fn parse_node_detail(v: &Value, node_name: &str) -> PopupContent {
    let peers = v
        .get("peers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let peer = peers.iter().find(|p| {
        p.get("peer_name")
            .and_then(Value::as_str)
            .map(|n| n == node_name)
            .unwrap_or(false)
    });

    let peer = match peer {
        Some(p) => p,
        None => return error_popup(&format!("node '{node_name}' not found")),
    };

    let caps = peer
        .get("capabilities")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|c| c.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "—".to_string());

    let node_section = PopupSection {
        label: "Node".to_string(),
        lines: vec![
            format!("Name:  {}", str_field(peer, "peer_name")),
            format!("Role:  {}", str_field(peer, "role")),
            format!("IP:    {}", str_field(peer, "tailscale_ip")),
            format!("OS:    {}", str_field(peer, "os")),
            format!("Caps:  {caps}"),
        ],
    };

    let online = peer.get("is_online").and_then(Value::as_bool).unwrap_or(false);
    let cpu = peer.get("cpu_percent").and_then(Value::as_f64).unwrap_or(0.0);
    let mem = peer.get("memory_percent").and_then(Value::as_f64).unwrap_or(0.0);
    let health_section = PopupSection {
        label: "Health".to_string(),
        lines: vec![
            format!("Online: {}", if online { "yes" } else { "no" }),
            format!("CPU:    {cpu:.1}%"),
            format!("Memory: {mem:.1}%"),
        ],
    };

    PopupContent {
        title: format!("Node: {node_name}"),
        sections: vec![node_section, health_section],
        actions: vec![('p', "Provision".to_string()), ('h', "Heartbeat".to_string())],
    }
}

/// GET {api_url}/api/mesh → find peer by name → PopupContent
pub async fn fetch_node_detail(
    client: &Client,
    api_url: &str,
    node_name: &str,
) -> PopupContent {
    let url = format!("{api_url}/api/mesh");
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(v) => parse_node_detail(&v, node_name),
            Err(e) => error_popup(&format!("parse error: {e}")),
        },
        Err(e) => error_popup(&format!("fetch error: {e}")),
    }
}

/// GET {api_url}/api/agents → find agent by name → PopupContent
pub async fn fetch_agent_detail(
    client: &Client,
    api_url: &str,
    agent_name: &str,
) -> PopupContent {
    let url = format!("{api_url}/api/agents");
    let v = match client.get(&url).send().await {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(v) => v,
            Err(e) => return error_popup(&format!("parse error: {e}")),
        },
        Err(e) => return error_popup(&format!("fetch error: {e}")),
    };

    let agents = v
        .get("running")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let agent = agents.iter().find(|a| {
        a.get("agent_id")
            .and_then(Value::as_str)
            .map(|n| n == agent_name)
            .unwrap_or(false)
    });

    let agent = match agent {
        Some(a) => a,
        None => return error_popup(&format!("agent '{agent_name}' not found")),
    };

    let agent_section = PopupSection {
        label: "Agent".to_string(),
        lines: vec![
            format!("Name: {}", str_field(agent, "agent_id")),
            format!("Type: {}", str_field(agent, "type")),
            format!("Host: {}", str_field(agent, "host")),
        ],
    };

    let task = agent.get("description").and_then(Value::as_str).unwrap_or("—");
    let status = agent.get("status").and_then(Value::as_str).unwrap_or("—");
    let activity_section = PopupSection {
        label: "Activity".to_string(),
        lines: vec![format!("Task:   {task}"), format!("Status: {status}")],
    };

    PopupContent {
        title: format!("Agent: {agent_name}"),
        sections: vec![agent_section, activity_section],
        actions: vec![('s', "Stop Agent".to_string())],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_node_detail_maps_all_fields() {
        let json = serde_json::json!({
            "peers": [
                {
                    "peer_name": "macProM1",
                    "role": "coordinator",
                    "tailscale_ip": "100.64.0.1",
                    "os": "darwin",
                    "capabilities": ["rust", "claude"],
                    "is_online": true,
                    "cpu_percent": 35.5,
                    "memory_percent": 60.2
                }
            ]
        });

        let popup = parse_node_detail(&json, "macProM1");
        assert_eq!(popup.title, "Node: macProM1");
        assert_eq!(popup.sections.len(), 2);
        assert_eq!(popup.sections[0].label, "Node");
        assert!(popup.sections[0].lines.iter().any(|l| l.contains("coordinator")));
        assert!(popup.sections[0].lines.iter().any(|l| l.contains("100.64.0.1")));
        assert!(popup.sections[0].lines.iter().any(|l| l.contains("darwin")));
        assert!(popup.sections[0].lines.iter().any(|l| l.contains("rust")));
        assert_eq!(popup.sections[1].label, "Health");
        assert!(popup.sections[1].lines.iter().any(|l| l.contains("yes")));
        assert!(popup.sections[1].lines.iter().any(|l| l.contains("35.5")));
        assert_eq!(popup.actions.len(), 2);
        assert_eq!(popup.actions[0], ('p', "Provision".to_string()));
        assert_eq!(popup.actions[1], ('h', "Heartbeat".to_string()));
    }

    #[test]
    fn parse_node_detail_returns_error_for_unknown_node() {
        let json = serde_json::json!({"peers": []});
        let popup = parse_node_detail(&json, "ghost-node");
        assert_eq!(popup.title, "Error");
        assert!(popup.sections[0].lines[0].contains("ghost-node"));
    }

    #[test]
    fn parse_node_detail_offline_node_shows_no() {
        let json = serde_json::json!({
            "peers": [{"peer_name": "worker1", "is_online": false}]
        });
        let popup = parse_node_detail(&json, "worker1");
        assert!(popup.sections[1].lines.iter().any(|l| l.contains("no")));
    }
}
