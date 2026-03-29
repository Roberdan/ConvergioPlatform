pub mod actions;
mod brain;
pub mod chat;
pub mod cost;
pub mod deliverables;
pub mod detail;
pub mod events;
pub(crate) mod plans;
pub mod project_tree;
pub mod projects;
pub mod workspace;
pub use actions::{mesh_heartbeat, mesh_provision, stop_agent};
pub use brain::{fetch_brain, parse_brain_response};
pub use cost::{fetch_cost, fetch_metrics_summary};
pub use deliverables::{fetch_deliverables, parse_deliverables_response};
pub use events::{fetch_events, parse_events_response};
pub use project_tree::{fetch_project_tree, parse_tree_response};
pub use projects::{fetch_projects, parse_projects_response};
pub use workspace::{fetch_workspaces, parse_workspaces_response};
pub use plans::{fetch_plans, fetch_all_tasks, fetch_tasks};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::tui::{AgentOrgNode, KpiData, MeshNode};

// --- API response shapes (match daemon endpoints) ---

#[allow(dead_code)]
#[derive(Deserialize)]
struct MeshPeer {
    peer_name: Option<String>,
    is_online: Option<bool>,
    role: Option<String>,
    cpu_percent: Option<f64>,
}

#[derive(Deserialize)]
struct AgentsResponse {
    running: Option<Vec<AgentRow>>,
}

#[derive(Deserialize)]
struct AgentRow {
    agent_id: Option<String>,
    #[serde(rename = "type")]
    agent_type: Option<String>,
    host: Option<String>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct OverviewResponse {
    plans_active: Option<i64>,
    agents_running: Option<i64>,
    today_tokens: Option<i64>,
    today_cost: Option<f64>,
    mesh_online: Option<i64>,
}

// --- Fetch functions ---

/// GET {api_url}/api/overview -> KpiData
pub async fn fetch_overview(client: &Client, api_url: &str) -> KpiData {
    let url = format!("{api_url}/api/overview");
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<OverviewResponse>().await {
            Ok(o) => KpiData {
                plans_active: o.plans_active.unwrap_or(0),
                agents_running: o.agents_running.unwrap_or(0),
                daily_tokens: o.today_tokens.unwrap_or(0),
                daily_cost: o.today_cost.unwrap_or(0.0),
                mesh_online: o.mesh_online.unwrap_or(0),
            },
            Err(_) => KpiData::default(),
        },
        Err(_) => KpiData::default(),
    }
}

/// GET {api_url}/api/mesh -> Vec<MeshNode>
pub async fn fetch_mesh(client: &Client, api_url: &str) -> Vec<MeshNode> {
    let url = format!("{api_url}/api/mesh");
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(v) => v.get("peers").and_then(|p| p.as_array()).map(|arr| {
                arr.iter().filter_map(|r| {
                    Some(MeshNode {
                        name: r.get("peer_name")?.as_str()?.to_string(),
                        online: r.get("is_online").and_then(|v| v.as_bool()).unwrap_or(false),
                        role: r.get("role").and_then(|v| v.as_str()).unwrap_or("worker").to_string(),
                        cpu_percent: r.get("cpu").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    })
                }).collect()
            }).unwrap_or_default(),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
}

/// GET {api_url}/api/agents -> Vec<AgentOrgNode>
pub async fn fetch_agents(client: &Client, api_url: &str) -> Vec<AgentOrgNode> {
    let url = format!("{api_url}/api/agents");
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<AgentsResponse>().await {
            Ok(r) => r
                .running
                .unwrap_or_default()
                .into_iter()
                .map(|a| AgentOrgNode {
                    name: a.agent_id.unwrap_or_default(),
                    role: a.agent_type.unwrap_or_default(),
                    host: a.host.unwrap_or_default(),
                    active_task: a.description,
                })
                .collect(),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
}

/// Fetch all TUI data in parallel, return updated TuiData.
pub async fn refresh_all(client: &Client, url: &str, data: &mut super::data::TuiData) {
    let (kpis, plans, tasks, mesh, agents, (brain_nodes, brain_kpi),
        cost_resp, summary, events, workspaces, deliverables, project_tree,
        project_list,
    ) = tokio::join!(
        fetch_overview(client, url), fetch_plans(client, url),
        fetch_all_tasks(client, url), fetch_mesh(client, url),
        fetch_agents(client, url), brain::fetch_brain(client, url),
        cost::fetch_cost(client, url), cost::fetch_metrics_summary(client, url),
        events::fetch_events(client, url), workspace::fetch_workspaces(client, url),
        deliverables::fetch_deliverables(client, url),
        project_tree::fetch_project_tree(client, url),
        projects::fetch_projects(client, url),
    );
    let has_brain_kpi = brain_kpi.daily_tokens > 0 || brain_kpi.daily_cost > 0.0;
    data.kpis = if has_brain_kpi { super::data::KpiData {
        daily_tokens: brain_kpi.daily_tokens, daily_cost: brain_kpi.daily_cost, ..kpis
    }} else { kpis };
    data.plans = plans; data.pipeline = tasks; data.mesh_nodes = mesh;
    data.agents = agents; data.brain_nodes = brain_nodes;
    data.events = events; data.workspaces = workspaces;
    data.deliverables = deliverables; data.project_tree = project_tree;
    data.projects = project_list;
    data.cost = super::data::CostData {
        by_model: cost_resp.by_model, by_project: cost_resp.by_project,
        by_date: cost_resp.by_date, summary,
    };
}
