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

// --- model inference ---

#[test]
fn defaults_model_feature_gets_codex() {
    let mut task = make_task("feature");
    apply_defaults(&mut task);
    assert_eq!(task.model.as_deref(), Some("codex"));
}

#[test]
fn defaults_model_fix_gets_codex() {
    let mut task = make_task("fix");
    apply_defaults(&mut task);
    assert_eq!(task.model.as_deref(), Some("codex"));
}

#[test]
fn defaults_model_planning_gets_opus() {
    let mut task = make_task("planning");
    apply_defaults(&mut task);
    assert_eq!(task.model.as_deref(), Some("opus"));
}

#[test]
fn defaults_model_analysis_gets_opus() {
    let mut task = make_task("analysis");
    apply_defaults(&mut task);
    assert_eq!(task.model.as_deref(), Some("opus"));
}

#[test]
fn defaults_model_doc_gets_sonnet() {
    let mut task = make_task("doc");
    apply_defaults(&mut task);
    assert_eq!(task.model.as_deref(), Some("sonnet"));
}

#[test]
fn defaults_model_chore_gets_haiku() {
    let mut task = make_task("chore");
    apply_defaults(&mut task);
    assert_eq!(task.model.as_deref(), Some("haiku"));
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
    assert_eq!(task.validator_agent.as_deref(), Some("compliance-validator"));
}

#[test]
fn defaults_validator_no_output_type_defaults_thor() {
    let mut task = make_task("feature");
    apply_defaults(&mut task);
    assert_eq!(task.validator_agent.as_deref(), Some("thor"));
}

#[test]
fn defaults_validator_explicit_not_overridden() {
    let mut task = make_task("feature");
    task.validator_agent = Some("custom-validator".to_string());
    apply_defaults(&mut task);
    assert_eq!(task.validator_agent.as_deref(), Some("custom-validator"));
}

// --- verify generation ---

#[test]
fn defaults_verify_generates_test_f_per_file() {
    let mut task = make_task("feature");
    task.files = vec!["src/main.rs".to_string(), "src/lib.rs".to_string()];
    apply_defaults(&mut task);
    assert_eq!(task.verify, vec!["test -f src/main.rs", "test -f src/lib.rs"]);
}

#[test]
fn defaults_verify_not_overridden_when_present() {
    let mut task = make_task("feature");
    task.files = vec!["src/main.rs".to_string()];
    task.verify = vec!["cargo check".to_string()];
    apply_defaults(&mut task);
    assert_eq!(task.verify, vec!["cargo check"]);
}

#[test]
fn defaults_verify_skipped_when_no_files() {
    let mut task = make_task("feature");
    apply_defaults(&mut task);
    assert!(task.verify.is_empty());
}

// --- effort inference ---

#[test]
fn defaults_effort_one_file_gives_level_1() {
    let mut task = make_task("feature");
    task.files = vec!["src/main.rs".to_string()];
    apply_defaults(&mut task);
    assert_eq!(task.effort_level, Some(1));
}

#[test]
fn defaults_effort_three_files_gives_level_2() {
    let mut task = make_task("feature");
    task.files = (0..3).map(|i| format!("src/f{i}.rs")).collect();
    apply_defaults(&mut task);
    assert_eq!(task.effort_level, Some(2));
}

#[test]
fn defaults_effort_five_files_gives_level_3() {
    let mut task = make_task("feature");
    task.files = (0..5).map(|i| format!("src/f{i}.rs")).collect();
    apply_defaults(&mut task);
    assert_eq!(task.effort_level, Some(3));
}

#[test]
fn defaults_effort_planning_with_no_files_gives_level_2() {
    let mut task = make_task("planning");
    apply_defaults(&mut task);
    assert_eq!(task.effort_level, Some(2));
}

#[test]
fn defaults_effort_chore_five_files_gives_level_3() {
    // file count wins over chore type when higher
    let mut task = make_task("chore");
    task.files = (0..5).map(|i| format!("f{i}")).collect();
    apply_defaults(&mut task);
    assert_eq!(task.effort_level, Some(3));
}

#[test]
fn defaults_effort_explicit_not_overridden() {
    let mut task = make_task("feature");
    task.effort_level = Some(3);
    apply_defaults(&mut task);
    assert_eq!(task.effort_level, Some(3));
}
