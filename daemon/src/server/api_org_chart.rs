//! GET /api/orgs/:slug/orgchart — render org structure as JSON or ASCII.

use crate::org::factory::{AgentSpec, Department, NightAgentSpec, OrgBlueprint};
use crate::org::orgchart::render_orgchart;
use crate::server::api_ipc::ensure_ipc_schema;
use crate::server::state::{query_one, query_rows, ApiError, ServerState};

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct OrgchartQuery {
    pub format: Option<String>,
}

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/orgs/:slug/orgchart", get(get_orgchart))
}

/// Build an OrgBlueprint from DB rows for rendering.
fn blueprint_from_db(
    org: &Value,
    members: &[Value],
    services: &[Value],
    plans: &[Value],
) -> OrgBlueprint {
    let slug = org["id"].as_str().unwrap_or("unknown").to_string();
    let mission = org["mission"].as_str().unwrap_or("").to_string();
    let ceo = org["ceo_agent"].as_str().unwrap_or("ceo").to_string();
    let budget = org["budget"].as_f64();

    // Group members by department to form departments.
    let mut dept_map: std::collections::BTreeMap<String, Vec<AgentSpec>> =
        std::collections::BTreeMap::new();
    for m in members {
        let dept_name = m["department"].as_str().unwrap_or("General").to_string();
        let agent_name = m["agent"].as_str().unwrap_or("unknown").to_string();
        let role = m["role"].as_str().unwrap_or("agent").to_string();
        dept_map.entry(dept_name).or_default().push(AgentSpec {
            name: agent_name,
            model: String::new(),
            capabilities: vec![role],
        });
    }
    let departments: Vec<Department> = dept_map
        .into_iter()
        .map(|(name, agents)| Department { name, agents })
        .collect();

    // Attach service names as a pseudo-"night agent" section for visibility.
    let night_agents: Vec<NightAgentSpec> = services
        .iter()
        .filter_map(|s| {
            let name = s["name"].as_str()?;
            Some(NightAgentSpec {
                name: name.to_string(),
                schedule: s["status"].as_str().unwrap_or("active").to_string(),
                time: s["endpoint"].as_str().unwrap_or("").to_string(),
                model: String::new(),
            })
        })
        .collect();

    let _plan_count = plans.len();
    OrgBlueprint {
        name: slug.clone(),
        slug,
        mission,
        repo_path: None,
        budget_usd: budget,
        ceo_agent: ceo,
        departments,
        night_agents,
    }
}

/// Build a JSON tree representation of the org structure.
fn orgchart_json(
    org: &Value,
    members: &[Value],
    services: &[Value],
    plans: &[Value],
) -> Value {
    let mut dept_map: std::collections::BTreeMap<String, Vec<Value>> =
        std::collections::BTreeMap::new();
    for m in members {
        let dept = m["department"].as_str().unwrap_or("General").to_string();
        dept_map.entry(dept).or_default().push(json!({
            "agent": m["agent"],
            "role": m["role"],
        }));
    }
    let departments: Vec<Value> = dept_map
        .into_iter()
        .map(|(name, agents)| json!({ "name": name, "agents": agents }))
        .collect();

    json!({
        "ok": true,
        "org": {
            "id": org["id"],
            "mission": org["mission"],
            "ceo_agent": org["ceo_agent"],
            "budget": org["budget"],
            "status": org["status"],
        },
        "departments": departments,
        "services": services,
        "plans": plans.iter().map(|p| json!({
            "id": p["id"], "title": p["title"], "status": p["status"],
        })).collect::<Vec<_>>(),
    })
}

async fn get_orgchart(
    State(state): State<ServerState>,
    Path(slug): Path<String>,
    Query(params): Query<OrgchartQuery>,
) -> Result<Response, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;

    let org = query_one(
        &conn,
        "SELECT id, mission, objectives, ceo_agent, budget, status
         FROM ipc_orgs WHERE id = ?1",
        rusqlite::params![slug],
    )?
    .ok_or_else(|| ApiError::not_found("org not found"))?;

    let members = query_rows(
        &conn,
        "SELECT agent, role, department FROM ipc_org_members
         WHERE org_id = ?1 ORDER BY joined_at",
        rusqlite::params![slug],
    )?;
    let services = query_rows(
        &conn,
        "SELECT name, endpoint, status FROM ipc_org_services
         WHERE org_id = ?1 ORDER BY registered_at",
        rusqlite::params![slug],
    )?;
    let plans = query_rows(
        &conn,
        "SELECT id, title, status FROM plans
         WHERE title LIKE '%' || ?1 || '%' ORDER BY id DESC LIMIT 10",
        rusqlite::params![slug],
    )?;

    if params.format.as_deref() == Some("ascii") {
        let bp = blueprint_from_db(&org, &members, &services, &plans);
        let ascii = render_orgchart(&bp);
        return Ok((
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            ascii,
        )
            .into_response());
    }

    let tree = orgchart_json(&org, &members, &services, &plans);
    Ok(Json(tree).into_response())
}
