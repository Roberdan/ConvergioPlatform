use std::env;

use reqwest::Client;
use serde::Deserialize;

use super::{AgentOrgNode, KpiData, MeshNode, PlanCard, TaskPipelineItem};

fn base_url() -> String {
    env::var("CONVERGIO_API_URL").unwrap_or_else(|_| "http://localhost:8420".to_string())
}

// --- API response shapes ---

#[derive(Deserialize)]
struct PlanRow {
    id: i64,
    name: String,
    status: String,
    tasks_done: Option<i64>,
    tasks_total: Option<i64>,
}

#[derive(Deserialize)]
struct TaskRow {
    task_id: Option<String>,
    title: Option<String>,
    status: Option<String>,
    agent: Option<String>,
}

#[derive(Deserialize)]
struct NodeRow {
    name: Option<String>,
    online: Option<bool>,
    active_tasks: Option<i64>,
    cpu_load: Option<i64>,
}

#[derive(Deserialize)]
struct AgentRow {
    name: Option<String>,
    role: Option<String>,
    host: Option<String>,
    active_task: Option<String>,
}

#[derive(Deserialize)]
struct OverviewResponse {
    plans_total: Option<i64>,
    tasks_total: Option<i64>,
    tasks_done: Option<i64>,
    nodes_online: Option<i64>,
    agents_active: Option<i64>,
}

// --- Fetch functions ---

pub async fn fetch_overview(client: &Client) -> KpiData {
    let url = format!("{}/api/v1/dashboard/overview", base_url());
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<OverviewResponse>().await {
            Ok(o) => KpiData {
                plans_total: o.plans_total.unwrap_or(0),
                tasks_total: o.tasks_total.unwrap_or(0),
                tasks_done: o.tasks_done.unwrap_or(0),
                nodes_online: o.nodes_online.unwrap_or(0),
                agents_active: o.agents_active.unwrap_or(0),
            },
            Err(_) => KpiData::default(),
        },
        Err(_) => KpiData::default(),
    }
}

pub async fn fetch_plans(client: &Client) -> Vec<PlanCard> {
    let url = format!("{}/api/v1/plans", base_url());
    let rows: Vec<PlanRow> = fetch_json(client, &url).await;
    rows.into_iter()
        .map(|r| PlanCard {
            id: r.id,
            name: r.name,
            status: r.status,
            tasks_done: r.tasks_done.unwrap_or(0),
            tasks_total: r.tasks_total.unwrap_or(0),
        })
        .collect()
}

pub async fn fetch_tasks(client: &Client) -> Vec<TaskPipelineItem> {
    let url = format!("{}/api/v1/plans/tasks", base_url());
    let rows: Vec<TaskRow> = fetch_json(client, &url).await;
    rows.into_iter()
        .map(|r| TaskPipelineItem {
            task_id: r.task_id.unwrap_or_default(),
            title: r.title.unwrap_or_default(),
            status: r.status.unwrap_or_default(),
            agent: r.agent.unwrap_or_default(),
        })
        .collect()
}

pub async fn fetch_mesh(client: &Client) -> Vec<MeshNode> {
    let url = format!("{}/api/v1/mesh/peers", base_url());
    let rows: Vec<NodeRow> = fetch_json(client, &url).await;
    rows.into_iter()
        .map(|r| MeshNode {
            name: r.name.unwrap_or_default(),
            online: r.online.unwrap_or(false),
            active_tasks: r.active_tasks.unwrap_or(0),
            cpu_load: r.cpu_load.unwrap_or(0),
        })
        .collect()
}

pub async fn fetch_agents(client: &Client) -> Vec<AgentOrgNode> {
    let url = format!("{}/api/v1/agents", base_url());
    let rows: Vec<AgentRow> = fetch_json(client, &url).await;
    rows.into_iter()
        .map(|r| AgentOrgNode {
            name: r.name.unwrap_or_default(),
            role: r.role.unwrap_or_default(),
            host: r.host.unwrap_or_default(),
            active_task: r.active_task,
        })
        .collect()
}

async fn fetch_json<T: serde::de::DeserializeOwned>(client: &Client, url: &str) -> Vec<T> {
    match client.get(url).send().await {
        Ok(resp) => resp.json::<Vec<T>>().await.unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}
