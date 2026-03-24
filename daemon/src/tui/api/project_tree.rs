// Project tree fetch — /api/project/{project}/tree

use reqwest::Client;
use serde::Deserialize;

use crate::tui::data::{ProjectTreeData, ProjectTreeNode};

#[derive(Deserialize)]
struct TreeResponse {
    project_name: Option<String>,
    total_tasks: Option<i64>,
    done_tasks: Option<i64>,
    plans: Option<Vec<TreePlan>>,
}

#[derive(Deserialize)]
struct TreePlan {
    id: Option<i64>,
    name: Option<String>,
    status: Option<String>,
    tasks_done: Option<i64>,
    tasks_total: Option<i64>,
    is_master: Option<bool>,
    depends_on: Option<String>,
    execution_mode: Option<String>,
    children: Option<Vec<TreePlan>>,
}

fn map_plan(p: TreePlan) -> ProjectTreeNode {
    let children = p
        .children
        .unwrap_or_default()
        .into_iter()
        .map(map_plan)
        .collect();
    ProjectTreeNode {
        id: p.id.unwrap_or(0),
        name: p.name.unwrap_or_default(),
        status: p.status.unwrap_or_default(),
        tasks_done: p.tasks_done.unwrap_or(0),
        tasks_total: p.tasks_total.unwrap_or(0),
        is_master: p.is_master.unwrap_or(false),
        depends_on: p.depends_on,
        execution_mode: p.execution_mode,
        children,
    }
}

/// Parse project tree JSON into ProjectTreeData. Public for testing.
pub fn parse_tree_response(v: &serde_json::Value) -> ProjectTreeData {
    match serde_json::from_value::<TreeResponse>(v.clone()) {
        Ok(r) => ProjectTreeData {
            project_name: r.project_name.unwrap_or_default(),
            total_tasks: r.total_tasks.unwrap_or(0),
            done_tasks: r.done_tasks.unwrap_or(0),
            plans: r.plans.unwrap_or_default().into_iter().map(map_plan).collect(),
        },
        Err(_) => ProjectTreeData::default(),
    }
}

/// GET {api_url}/api/project/convergio/tree -> ProjectTreeData
pub async fn fetch_project_tree(client: &Client, api_url: &str) -> ProjectTreeData {
    let url = format!("{api_url}/api/project/convergio/tree");
    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(v) => parse_tree_response(&v),
            Err(_) => ProjectTreeData::default(),
        },
        Err(_) => ProjectTreeData::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tree_with_master_and_children() {
        let json = serde_json::json!({
            "project_name": "convergio",
            "total_tasks": 831,
            "done_tasks": 413,
            "plans": [
                {
                    "id": 711, "name": "Convergio Vision", "status": "draft",
                    "tasks_done": 0, "tasks_total": 0, "is_master": true,
                    "execution_mode": "mixed",
                    "children": [
                        {"id": 719, "name": "Plan H0", "status": "draft",
                         "tasks_done": 0, "tasks_total": 8, "depends_on": null},
                        {"id": 712, "name": "Plan H", "status": "draft",
                         "tasks_done": 0, "tasks_total": 7, "depends_on": "719"}
                    ]
                },
                {"id": 123, "name": "Old Plan", "status": "done",
                 "tasks_done": 5, "tasks_total": 5}
            ]
        });
        let tree = parse_tree_response(&json);
        assert_eq!(tree.project_name, "convergio");
        assert_eq!(tree.total_tasks, 831);
        assert_eq!(tree.done_tasks, 413);
        assert_eq!(tree.plans.len(), 2);

        let master = &tree.plans[0];
        assert!(master.is_master);
        assert_eq!(master.children.len(), 2);
        assert_eq!(master.execution_mode.as_deref(), Some("mixed"));

        let child = &master.children[1];
        assert_eq!(child.id, 712);
        assert_eq!(child.depends_on.as_deref(), Some("719"));

        let orphan = &tree.plans[1];
        assert!(!orphan.is_master);
        assert!(orphan.children.is_empty());
    }

    #[test]
    fn parse_tree_empty_response() {
        let json = serde_json::json!({});
        let tree = parse_tree_response(&json);
        assert!(tree.plans.is_empty());
        assert_eq!(tree.total_tasks, 0);
    }

    #[test]
    fn parse_tree_null_children() {
        let json = serde_json::json!({
            "project_name": "test",
            "plans": [{"id": 1, "name": "Solo", "status": "doing"}]
        });
        let tree = parse_tree_response(&json);
        assert_eq!(tree.plans.len(), 1);
        assert!(tree.plans[0].children.is_empty());
    }
}
