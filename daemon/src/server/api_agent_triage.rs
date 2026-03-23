// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// POST /api/agents/triage — score agents from agent_catalog against a problem.

use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

/// When best triage score is below this, suggest creating a new agent.
const SUGGEST_CREATION_THRESHOLD: f64 = 0.3;

#[derive(Debug, Deserialize)]
pub struct TriageRequest {
    pub problem_description: String,
    pub domain: Option<String>,
}

/// Agent row from DB, used for scoring.
pub struct AgentRow {
    pub name: String,
    pub category: String,
    pub description: String,
}

/// Score an agent against a problem description and optional domain.
///
/// Scoring rules:
/// - Exact category match with domain = 1.0
/// - Partial keyword match (word appears in description or category) = 0.5
/// - Default (no match) = 0.1
pub fn score_agent(agent: &AgentRow, words: &[String], domain: &Option<String>) -> f64 {
    // Exact category match
    if let Some(ref dom) = domain {
        if agent.category.eq_ignore_ascii_case(dom) {
            return 1.0;
        }
    }

    let desc_lower = agent.description.to_lowercase();
    let cat_lower = agent.category.to_lowercase();

    for word in words {
        if word.len() < 3 {
            continue; // skip short words like "a", "is", "to"
        }
        if desc_lower.contains(word) || cat_lower.contains(word) {
            return 0.5;
        }
    }

    0.1
}

/// Request body for POST /api/agents/scaffold.
#[derive(Debug, Deserialize)]
pub struct ScaffoldRequest {
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub domain: Option<String>,
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/agents/triage", post(handle_triage))
        .route("/api/agents/scaffold", post(handle_scaffold))
}

async fn handle_triage(
    State(state): State<ServerState>,
    Json(body): Json<TriageRequest>,
) -> Result<Json<Value>, ApiError> {
    if body.problem_description.trim().is_empty() {
        return Err(ApiError::bad_request("problem_description is required"));
    }

    let conn = state.get_conn()?;

    // Fetch all agents from catalog
    let mut stmt = conn
        .prepare("SELECT name, category, description FROM agent_catalog")
        .map_err(|e| ApiError::internal(format!("prepare: {e}")))?;

    let agents: Vec<AgentRow> = stmt
        .query_map([], |row| {
            Ok(AgentRow {
                name: row.get(0)?,
                category: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                description: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            })
        })
        .map_err(|e| ApiError::internal(format!("query: {e}")))?
        .filter_map(|r| r.ok())
        .collect();

    // Tokenize problem description
    let words: Vec<String> = body
        .problem_description
        .to_lowercase()
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|w| !w.is_empty())
        .collect();

    // Score and sort
    let mut scored: Vec<(String, String, String, f64)> = agents
        .iter()
        .map(|a| {
            let s = score_agent(a, &words, &body.domain);
            (a.name.clone(), a.category.clone(), a.description.clone(), s)
        })
        .collect();

    scored.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(15);

    // Snapshot for threshold check before consuming via into_iter
    let scored_snapshot = scored.clone();

    let suggestions: Vec<Value> = scored
        .into_iter()
        .map(|(name, category, description, score)| {
            json!({
                "name": name,
                "category": category,
                "description": description,
                "score": score,
            })
        })
        .collect();

    let best_score = scored_snapshot.first().map(|s| s.3).unwrap_or(0.0);
    let suggest_creation = best_score < SUGGEST_CREATION_THRESHOLD;
    let mut result =
        json!({ "ok": true, "suggestions": suggestions, "suggest_creation": suggest_creation });

    if suggest_creation {
        // Provide a scaffold hint derived from the problem description
        let suggested_name = body
            .problem_description
            .split_whitespace()
            .take(3)
            .map(|w| {
                w.trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase()
            })
            .collect::<Vec<_>>()
            .join("-");
        result["scaffold_hint"] = json!({
            "name": if suggested_name.is_empty() { "new-agent".to_string() } else { suggested_name },
            "category": body.domain.clone().unwrap_or_else(|| "general".to_string()),
            "description": body.problem_description.clone(),
            "domain": body.domain.clone().unwrap_or_else(|| "general".to_string()),
        });
    }

    Ok(Json(result))
}

/// POST /api/agents/scaffold — generate an agent .md template from metadata.
async fn handle_scaffold(Json(body): Json<ScaffoldRequest>) -> Result<Json<Value>, ApiError> {
    if body.name.trim().is_empty() {
        return Err(ApiError::bad_request("name is required"));
    }
    let category = body.category.as_deref().unwrap_or("general");
    let description = body
        .description
        .as_deref()
        .unwrap_or("TODO: describe this agent");
    let domain = body.domain.as_deref().unwrap_or("general");

    let markdown = format!(
        "---\nname: {name}\ndescription: \"{description}\"\nmodel: claude-sonnet-4-6\ntools:\n  - view\n  - edit\n  - bash\n---\n\n# {name}\n\n**Role:** {description}\n\n## Domain\n\n{domain}\n\n## Category\n\n{category}\n\n## Capabilities\n\n- TODO: list key capabilities\n\n## Constraints\n\n- Follow the Convergio Constitution\n- Max 250 lines per file\n",
        name = body.name.trim(),
        description = description,
        domain = domain,
        category = category,
    );

    Ok(Json(
        json!({ "ok": true, "name": body.name.trim(), "markdown": markdown }),
    ))
}

#[cfg(test)]
#[path = "api_agent_triage_tests.rs"]
mod tests;
