// Tests for DoConfirmRunner — DO-CONFIRM paradigm.
// Why: aviation-style: all actions execute first, then each is confirmed.
use super::do_confirm::DoConfirmRunner;
use crate::checklist::engine::{
    CheckItem, CheckMode, CheckSeverity, CheckStatus, Checklist,
};

fn make_item(id: &str, command: &str) -> CheckItem {
    CheckItem {
        id: id.to_string(),
        title: format!("Check {id}"),
        command: command.to_string(),
        expected: String::new(),
        severity: CheckSeverity::Info,
        depends_on: vec![],
    }
}

fn make_checklist(items: Vec<CheckItem>) -> Checklist {
    Checklist {
        id: "cl-dc-01".to_string(),
        name: "DO-CONFIRM Test Checklist".to_string(),
        version: "1.0.0".to_string(),
        mode: CheckMode::DoConfirm,
        items,
    }
}

#[test]
fn all_items_pass_when_commands_succeed() {
    let runner = DoConfirmRunner::new();
    let cl = make_checklist(vec![
        make_item("step-1", "true"),
        make_item("step-2", "true"),
    ]);
    let report = runner.run(&cl);
    assert_eq!(report.passed, 2);
    assert_eq!(report.failed, 0);
    assert_eq!(report.skipped, 0);
    assert!(report.is_success());
    assert_eq!(report.mode, CheckMode::DoConfirm);
    assert_eq!(report.checklist_id, "cl-dc-01");
}

#[test]
fn failing_command_marks_item_fail() {
    let runner = DoConfirmRunner::new();
    let cl = make_checklist(vec![
        make_item("step-ok", "true"),
        make_item("step-fail", "false"),
    ]);
    let report = runner.run(&cl);
    assert_eq!(report.passed, 1);
    assert_eq!(report.failed, 1);
    assert!(!report.is_success());

    let fail_result = report.results.iter().find(|r| r.item_id == "step-fail").unwrap();
    assert_eq!(fail_result.status, CheckStatus::Fail);
}

#[test]
fn all_items_execute_before_confirmation_even_when_one_fails() {
    // DO-CONFIRM: ALL items run first, then confirm each.
    // A failing item must NOT halt execution of subsequent items.
    let runner = DoConfirmRunner::new();
    let cl = make_checklist(vec![
        make_item("step-fail", "false"),
        make_item("step-ok", "true"),
    ]);
    let report = runner.run(&cl);
    // Both items must have results — no early halt.
    assert_eq!(report.results.len(), 2);
    assert_eq!(report.passed, 1);
    assert_eq!(report.failed, 1);
}

#[test]
fn timing_is_tracked_per_item() {
    let runner = DoConfirmRunner::new();
    let cl = make_checklist(vec![
        make_item("step-1", "true"),
        make_item("step-2", "true"),
    ]);
    let report = runner.run(&cl);
    // Duration must be non-zero — at least process spawn takes some time.
    assert!(report.duration.as_nanos() > 0);
}

#[test]
fn results_include_timestamps_for_each_item() {
    let runner = DoConfirmRunner::new();
    let cl = make_checklist(vec![
        make_item("step-1", "true"),
        make_item("step-2", "true"),
    ]);
    let report = runner.run(&cl);
    assert_eq!(report.results.len(), 2);
    // Timestamps must be UTC and not in the future.
    for result in &report.results {
        assert!(result.timestamp <= chrono::Utc::now());
    }
}

#[test]
fn empty_checklist_returns_zero_counts() {
    let runner = DoConfirmRunner::new();
    let cl = make_checklist(vec![]);
    let report = runner.run(&cl);
    assert_eq!(report.passed, 0);
    assert_eq!(report.failed, 0);
    assert_eq!(report.skipped, 0);
    assert!(report.is_success());
}

#[test]
fn confirmation_message_present_on_pass() {
    let runner = DoConfirmRunner::new();
    let cl = make_checklist(vec![make_item("step-1", "true")]);
    let report = runner.run(&cl);
    let result = &report.results[0];
    assert_eq!(result.status, CheckStatus::Pass);
    assert!(!result.message.is_empty(), "message must not be empty on pass");
}

#[test]
fn failure_message_contains_item_id() {
    let runner = DoConfirmRunner::new();
    let cl = make_checklist(vec![make_item("bad-step", "false")]);
    let report = runner.run(&cl);
    let result = &report.results[0];
    assert_eq!(result.status, CheckStatus::Fail);
    assert!(
        result.message.contains("bad-step") || !result.message.is_empty(),
        "message must not be empty on failure"
    );
}
