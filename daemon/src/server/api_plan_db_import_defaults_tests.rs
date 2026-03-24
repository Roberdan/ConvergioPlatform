// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for api_plan_db_import_defaults module.

use super::*;
use crate::server::api_plan_db_import_parsers::TaskSpec;

fn make_task(task_type: &str) -> TaskSpec {
    TaskSpec {
        id: "T1".to_string(),
        title: "Test".to_string(),
        priority: "P1".to_string(),
        task_type: task_type.to_string(),
        description: None,
        test_criteria: None,
        model: None,
        assignee: None,
        output_type: None,
        validator_agent: None,
        files: vec![],
        verify: vec![],
        effort_level: None,
    }
}

fn assert_model(task_type: &str, expected: &str) {
    let mut task = make_task(task_type);
    apply_defaults(&mut task);
    assert_eq!(task.model.as_deref(), Some(expected), "type={task_type}");
}

// --- model inference ---

#[test]
fn defaults_model_opus_types() {
    for t in &["test", "planning", "analysis", "review"] {
        assert_model(t, "claude-opus-4.6");
    }
}

#[test]
fn defaults_model_codex_types() {
    for t in &[
        "feature",
        "fix",
        "refactor",
        "config",
        "documentation",
        "docs",
        "doc",
        "chore",
    ] {
        assert_model(t, "gpt-5.3-codex");
    }
}

#[test]
fn defaults_model_explicit_not_overridden() {
    let mut task = make_task("feature");
    task.model = Some("gpt4".to_string());
    apply_defaults(&mut task);
    assert_eq!(task.model.as_deref(), Some("gpt4"));
}

// --- validator inference ---

#[test]
fn defaults_validator_pr_gets_thor() {
    let mut task = make_task("feature");
    task.output_type = Some("pr".to_string());
    apply_defaults(&mut task);
    assert_eq!(task.validator_agent.as_deref(), Some("thor"));
}

#[test]
fn defaults_validator_no_output_type_defaults_thor() {
    let mut task = make_task("feature");
    apply_defaults(&mut task);
    assert_eq!(task.validator_agent.as_deref(), Some("thor"));
}

#[test]
fn defaults_validator_document_gets_doc_validator() {
    let mut task = make_task("doc");
    task.output_type = Some("document".to_string());
    apply_defaults(&mut task);
    assert_eq!(task.validator_agent.as_deref(), Some("doc-validator"));
}

#[test]
fn defaults_validator_analysis_gets_strategy_validator() {
    let mut task = make_task("analysis");
    task.output_type = Some("analysis".to_string());
    apply_defaults(&mut task);
    assert_eq!(task.validator_agent.as_deref(), Some("strategy-validator"));
}

#[test]
fn defaults_validator_design_gets_design_validator() {
    let mut task = make_task("feature");
    task.output_type = Some("design".to_string());
    apply_defaults(&mut task);
    assert_eq!(task.validator_agent.as_deref(), Some("design-validator"));
}

#[test]
fn defaults_validator_legal_gets_compliance_validator() {
    let mut task = make_task("chore");
    task.output_type = Some("legal_opinion".to_string());
    apply_defaults(&mut task);
    assert_eq!(
        task.validator_agent.as_deref(),
        Some("compliance-validator")
    );
}

#[test]
fn defaults_validator_explicit_not_overridden() {
    let mut task = make_task("feature");
    task.validator_agent = Some("custom-validator".to_string());
    apply_defaults(&mut task);
    assert_eq!(task.validator_agent.as_deref(), Some("custom-validator"));
}

// --- output type default ---

#[test]
fn defaults_output_type_none_becomes_pr() {
    let mut task = make_task("feature");
    apply_defaults(&mut task);
    assert_eq!(task.output_type.as_deref(), Some("pr"));
}

#[test]
fn defaults_output_type_explicit_not_overridden() {
    let mut task = make_task("feature");
    task.output_type = Some("document".to_string());
    apply_defaults(&mut task);
    assert_eq!(task.output_type.as_deref(), Some("document"));
}

#[path = "api_plan_db_import_defaults_tests_remaining.rs"]
mod remaining;
