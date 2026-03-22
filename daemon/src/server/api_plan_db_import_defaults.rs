// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Smart import defaults: infer model, validator_agent, verify, effort from task spec.

use super::api_plan_db_import_parsers::TaskSpec;

/// Apply smart defaults to tasks in-place before DB insert.
/// Rules:
///   - model: feature/fix→codex, planning/analysis→opus, docs/doc→sonnet, chore→haiku
///   - validator_agent: pr→thor, document→doc-validator, analysis→strategy-validator,
///     design→design-validator, legal_opinion→compliance-validator
///   - verify: if files present and verify empty, generate `test -f <file>` per file
///   - effort_level: 1 file→1, 2-4→2, 5+→3 (type also factors in: chore→1, planning→3)
pub fn apply_defaults(task: &mut TaskSpec) {
    apply_model_default(task);
    apply_validator_default(task);
    apply_verify_default(task);
    apply_effort_default(task);
}

fn apply_model_default(task: &mut TaskSpec) {
    if task.model.is_some() {
        return;
    }
    task.model = Some(infer_model(&task.task_type).to_string());
}

pub fn infer_model(task_type: &str) -> &'static str {
    match task_type {
        "feature" | "fix" | "bug" | "refactor" | "test" | "config" => "codex",
        "planning" | "analysis" | "review" => "opus",
        "documentation" | "docs" | "doc" => "sonnet",
        _ => "haiku", // chore, other
    }
}

fn apply_validator_default(task: &mut TaskSpec) {
    if task.validator_agent.is_some() {
        return;
    }
    let output = task.output_type.as_deref().unwrap_or("pr");
    task.validator_agent = Some(infer_validator(output).to_string());
}

pub fn infer_validator(output_type: &str) -> &'static str {
    match output_type {
        "pr" | "review" => "thor",
        "document" | "presentation" => "doc-validator",
        "analysis" | "plan" => "strategy-validator",
        "design" => "design-validator",
        "legal_opinion" => "compliance-validator",
        _ => "thor",
    }
}

fn apply_verify_default(task: &mut TaskSpec) {
    // Only generate if files present and verify is empty
    if task.files.is_empty() || !task.verify.is_empty() {
        return;
    }
    task.verify = task.files.iter().map(|f| format!("test -f {f}")).collect();
}

fn apply_effort_default(task: &mut TaskSpec) {
    if task.effort_level.is_some() {
        return;
    }
    task.effort_level = Some(infer_effort(&task.task_type, task.files.len()));
}

pub fn infer_effort(task_type: &str, file_count: usize) -> i64 {
    // Type override: planning/analysis always medium-high
    let type_effort = match task_type {
        "planning" | "analysis" => 2i64,
        "chore" => 1i64,
        _ => 0, // defer to file count
    };
    let file_effort = match file_count {
        0 | 1 => 1i64,
        2..=4 => 2,
        _ => 3,
    };
    // Take the higher of type-based and file-based effort
    type_effort.max(file_effort)
}

#[cfg(test)]
#[path = "api_plan_db_import_defaults_tests.rs"]
mod tests;
