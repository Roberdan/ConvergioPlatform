//! GET /api/orgs/chart — global orgchart showing all orgs in one view.

use crate::org::orgchart_global::{render_global_orgchart, OrgSummary};
use crate::server::api_ipc::ensure_ipc_schema;
use crate::server::state::{query_rows, ApiError, ServerState};

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/orgs/chart", get(get_global_orgchart))
}

/// Determine org_type: vertical if repo_path is set or mission hints at code.
fn classify_org(org: &Value) -> String {
    let mission = org["mission"].as_str().unwrap_or("").to_lowercase();
    let has_repo = org["repo_path"].as_str().map_or(false, |r| !r.is_empty());
    let code_keywords = ["software", "code", "platform", "daemon", "app"];
    if has_repo || code_keywords.iter().any(|k| mission.contains(k)) {
        "vertical".into()
    } else {
        "horizontal".into()
    }
}

async fn get_global_orgchart(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;

    // Fetch all orgs
    let orgs = query_rows(
        &conn,
        "SELECT id, mission, ceo_agent, budget, status FROM ipc_orgs ORDER BY id",
        rusqlite::params![],
    )?;

    let mut summaries: Vec<OrgSummary> = Vec::new();
    let mut total_agents: usize = 0;
    let mut total_plans: usize = 0;

    for org in &orgs {
        let slug = org["id"].as_str().unwrap_or("unknown").to_string();
        let name = slug.clone();
        let status = org["status"].as_str().unwrap_or("unknown").to_string();
        let org_type = classify_org(org);

        // Count members for this org
        let members = query_rows(
            &conn,
            "SELECT agent FROM ipc_org_members WHERE org_id = ?1",
            rusqlite::params![slug],
        )?;
        let agent_count = members.len();

        // Count plans for this org
        let plans = query_rows(
            &conn,
            "SELECT id FROM plans WHERE org_id = ?1",
            rusqlite::params![slug],
        )?;
        let plan_count = plans.len();

        total_agents += agent_count;
        total_plans += plan_count;

        summaries.push(OrgSummary {
            slug,
            name,
            org_type,
            agent_count,
            plan_count,
            status,
        });
    }

    let mesh_summary = build_mesh_summary(&state);
    let chart = render_global_orgchart(&summaries, &mesh_summary, total_agents, total_plans);

    Ok(Json(json!({
        "ok": true,
        "chart": chart,
        "orgs": summaries,
    })))
}

/// Build a short mesh description from env or default.
fn build_mesh_summary(state: &ServerState) -> String {
    let conn = match state.get_conn() {
        Ok(c) => c,
        Err(_) => return "unknown".into(),
    };
    let peers = query_rows(
        &conn,
        "SELECT name FROM mesh_peers ORDER BY name LIMIT 5",
        rusqlite::params![],
    );
    match peers {
        Ok(rows) if !rows.is_empty() => {
            let names: Vec<String> = rows
                .iter()
                .filter_map(|r| r["name"].as_str().map(String::from))
                .collect();
            names.join(" <-> ")
        }
        _ => "standalone".into(),
    }
}
