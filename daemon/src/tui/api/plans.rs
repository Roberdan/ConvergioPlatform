// Plan/task fetch functions extracted from mod.rs.
// Why: keep mod.rs ≤250 lines per CONSTITUTION Article V.
use crate::tui::{PlanCard, TaskPipelineItem};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

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

/// GET {api_url}/api/plan-db/list -> Vec<PlanCard>
pub async fn fetch_plans(client: &Client, api_url: &str) -> Vec<PlanCard> {
    let url = format!("{api_url}/api/plan-db/list");
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

/// GET {api_url}/api/plan/{plan_id} -> tasks for a specific plan
#[allow(dead_code)]
pub async fn fetch_tasks(client: &Client, plan_id: i64, api_url: &str) -> Vec<TaskPipelineItem> {
    let url = format!("{api_url}/api/plan/{plan_id}");
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

/// GET {api_url}/api/mission -> all active tasks across plans (pipeline view)
pub async fn fetch_all_tasks(client: &Client, api_url: &str) -> Vec<TaskPipelineItem> {
    let url = format!("{api_url}/api/mission");
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
