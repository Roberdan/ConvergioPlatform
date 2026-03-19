use std::env;

use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use super::{AgentOrgNode, KpiData, MeshNode, PlanCard, TaskPipelineItem};

fn base_url() -> String {
    env::var("CONVERGIO_API_URL").unwrap_or_else(|_| "http://localhost:8420".to_string())
}

// --- API response shapes (match daemon endpoints) ---

#[derive(Deserialize)]
struct PlanListResponse {
    plans: Option<Vec<PlanRow>>,
}

#[derive(Deserialize)]
struct PlanRow {
    id: Option<i64>,
    name: Option<String>,
    status: Option<String>,
    tasks_done: Option<i64>,
    tasks_total: Option<i64>,
}

#[derive(Deserialize)]
struct MissionResponse {
    plans: Option<Vec<MissionPlan>>,
}

#[derive(Deserialize)]
struct MissionPlan {
    tasks: Option<Vec<TaskRow>>,
}

#[derive(Deserialize)]
struct TaskRow {
    task_id: Option<String>,
    title: Option<String>,
    status: Option<String>,
    executor_agent: Option<String>,
}

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

/// GET /api/overview -> KpiData
pub async fn fetch_overview(client: &Client) -> KpiData {
    let url = format!("{}/api/overview", base_url());
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

/// GET /api/plan-db/list -> Vec<PlanCard>
pub async fn fetch_plans(client: &Client) -> Vec<PlanCard> {
    let url = format!("{}/api/plan-db/list", base_url());
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<PlanListResponse>().await {
            Ok(r) => r
                .plans
                .unwrap_or_default()
                .into_iter()
                .map(|p| PlanCard {
                    id: p.id.unwrap_or(0),
                    name: p.name.unwrap_or_default(),
                    status: p.status.unwrap_or_default(),
                    tasks_done: p.tasks_done.unwrap_or(0),
                    tasks_total: p.tasks_total.unwrap_or(0),
                })
                .collect(),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
}

/// GET /api/plan/:plan_id -> tasks for a specific plan
pub async fn fetch_tasks(client: &Client, plan_id: i64) -> Vec<TaskPipelineItem> {
    let url = format!("{}/api/plan/{plan_id}", base_url());
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(v) => v
                .get("tasks")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| serde_json::from_value::<TaskRow>(t.clone()).ok())
                        .map(|t| TaskPipelineItem {
                            task_id: t.task_id.unwrap_or_default(),
                            title: t.title.unwrap_or_default(),
                            status: t.status.unwrap_or_default(),
                            agent: t.executor_agent.unwrap_or_default(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
}

/// GET /api/mission -> all active tasks across plans (pipeline view)
pub async fn fetch_all_tasks(client: &Client) -> Vec<TaskPipelineItem> {
    let url = format!("{}/api/mission", base_url());
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<MissionResponse>().await {
            Ok(r) => r
                .plans
                .unwrap_or_default()
                .into_iter()
                .flat_map(|p| p.tasks.unwrap_or_default())
                .map(|t| TaskPipelineItem {
                    task_id: t.task_id.unwrap_or_default(),
                    title: t.title.unwrap_or_default(),
                    status: t.status.unwrap_or_default(),
                    agent: t.executor_agent.unwrap_or_default(),
                })
                .collect(),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
}

/// GET /api/mesh -> Vec<MeshNode>
pub async fn fetch_mesh(client: &Client) -> Vec<MeshNode> {
    let url = format!("{}/api/mesh", base_url());
    let rows: Vec<MeshPeer> = fetch_json(client, &url).await;
    rows.into_iter()
        .map(|r| MeshNode {
            name: r.peer_name.unwrap_or_default(),
            online: r.is_online.unwrap_or(false),
            role: r.role.unwrap_or_else(|| "worker".to_string()),
            cpu_percent: r.cpu_percent.unwrap_or(0.0),
        })
        .collect()
}

/// GET /api/agents -> Vec<AgentOrgNode>
pub async fn fetch_agents(client: &Client) -> Vec<AgentOrgNode> {
    let url = format!("{}/api/agents", base_url());
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

async fn fetch_json<T: serde::de::DeserializeOwned>(client: &Client, url: &str) -> Vec<T> {
    match client.get(url).send().await {
        Ok(resp) => resp.json::<Vec<T>>().await.unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}
