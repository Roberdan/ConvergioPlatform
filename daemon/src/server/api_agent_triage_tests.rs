// Tests for agent triage scoring — extracted from api_agent_triage.rs (Plan F, T5-02).

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

#[test]
fn suggest_creation_threshold_is_below_point_three() {
    assert!(0.1 < SUGGEST_CREATION_THRESHOLD);
    assert!(0.5 >= SUGGEST_CREATION_THRESHOLD);
}
