// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// POST /api/agents/triage — score agents from agent_catalog against a problem.

use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

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

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/agents/triage", post(handle_triage))
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
            (
                a.name.clone(),
                a.category.clone(),
                a.description.clone(),
                s,
            )
        })
        .collect();

    scored.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(15);

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

    Ok(Json(json!({ "ok": true, "suggestions": suggestions })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent(name: &str, category: &str, description: &str) -> AgentRow {
        AgentRow {
            name: name.to_string(),
            category: category.to_string(),
            description: description.to_string(),
        }
    }

    #[test]
    fn exact_domain_match_scores_highest() {
        let agent = make_agent("validate", "core", "Thor quality validation");
        let words = vec!["debugging".to_string()];
        let domain = Some("core".to_string());
        assert!((score_agent(&agent, &words, &domain) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn keyword_match_scores_partial() {
        let agent = make_agent("adversarial-debugger", "technical", "Deep debugging");
        let words = vec!["debugging".to_string()];
        let domain = None;
        assert!((score_agent(&agent, &words, &domain) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn no_match_scores_default() {
        let agent = make_agent("check", "core", "Quick checks");
        let words = vec!["security".to_string(), "vulnerability".to_string()];
        let domain = None;
        assert!((score_agent(&agent, &words, &domain) - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn short_words_are_ignored() {
        let agent = make_agent("Convergio", "technical", "Platform control plane expert");
        // "is" and "a" are too short (<3 chars) to match
        let words = vec!["is".to_string(), "a".to_string()];
        let domain = None;
        assert!((score_agent(&agent, &words, &domain) - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn category_keyword_match_scores_partial() {
        let agent = make_agent("code-reviewer", "technical", "Code review");
        let words = vec!["technical".to_string()];
        let domain = None;
        assert!((score_agent(&agent, &words, &domain) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn domain_match_is_case_insensitive() {
        let agent = make_agent("planner", "core", "Plan creation");
        let words = vec![];
        let domain = Some("CORE".to_string());
        assert!((score_agent(&agent, &words, &domain) - 1.0).abs() < f64::EPSILON);
    }
}
