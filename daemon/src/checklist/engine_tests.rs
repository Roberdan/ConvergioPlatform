// Unit tests for ChecklistEngine types and trait contract.
use super::engine::{
    CheckItem, CheckMode, CheckResult, CheckSeverity, CheckStatus, Checklist, ChecklistEngine,
    ChecklistError, ExecutionReport,
};
use chrono::Utc;
use std::path::Path;
use std::time::Duration;

pub fn make_item(id: &str) -> CheckItem {
    CheckItem {
        id: id.to_string(),
        title: format!("Check {id}"),
        command: "echo ok".to_string(),
        expected: "ok".to_string(),
        severity: CheckSeverity::Info,
        depends_on: vec![],
    }
}

pub fn make_checklist(mode: CheckMode) -> Checklist {
    Checklist {
        id: "cl-001".to_string(),
        name: "Smoke Checklist".to_string(),
        version: "1.0.0".to_string(),
        mode,
        items: vec![make_item("step-1"), make_item("step-2")],
    }
}

// --- Type construction tests ---

#[test]
fn check_item_roundtrip_serde() {
    let item = make_item("item-a");
    let json = serde_json::to_string(&item).expect("serialize");
    let back: CheckItem = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.id, "item-a");
    assert_eq!(back.severity, CheckSeverity::Info);
}

#[test]
fn checklist_stores_items() {
    let cl = make_checklist(CheckMode::DoConfirm);
    assert_eq!(cl.items.len(), 2);
    assert_eq!(cl.mode, CheckMode::DoConfirm);
}

#[test]
fn check_result_timestamp_is_utc() {
    let result = CheckResult {
        item_id: "step-1".to_string(),
        status: CheckStatus::Pass,
        message: "passed".to_string(),
        timestamp: Utc::now(),
    };
    assert!(result.timestamp <= Utc::now());
}

#[test]
fn execution_report_counts_correctly() {
    let results = vec![
        CheckResult {
            item_id: "s1".to_string(),
            status: CheckStatus::Pass,
            message: "ok".to_string(),
            timestamp: Utc::now(),
        },
        CheckResult {
            item_id: "s2".to_string(),
            status: CheckStatus::Fail,
            message: "fail".to_string(),
            timestamp: Utc::now(),
        },
        CheckResult {
            item_id: "s3".to_string(),
            status: CheckStatus::Skip,
            message: "skipped".to_string(),
            timestamp: Utc::now(),
        },
    ];
    let report = ExecutionReport::from_results(
        "cl-001".to_string(),
        CheckMode::ReadDo,
        results,
        Duration::from_millis(42),
    );
    assert_eq!(report.passed, 1);
    assert_eq!(report.failed, 1);
    assert_eq!(report.skipped, 1);
    assert!(!report.is_success());
}

#[test]
fn execution_report_success_when_no_failures() {
    let results = vec![CheckResult {
        item_id: "s1".to_string(),
        status: CheckStatus::Pass,
        message: "ok".to_string(),
        timestamp: Utc::now(),
    }];
    let report = ExecutionReport::from_results(
        "cl-001".to_string(),
        CheckMode::DoConfirm,
        results,
        Duration::from_millis(10),
    );
    assert!(report.is_success());
}

#[test]
fn check_mode_serde_roundtrip() {
    let modes = [CheckMode::DoConfirm, CheckMode::ReadDo];
    for mode in &modes {
        let json = serde_json::to_string(mode).expect("serialize");
        let back: CheckMode = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(&back, mode);
    }
}

#[test]
fn checklist_depends_on_empty_by_default() {
    let item = make_item("x");
    assert!(item.depends_on.is_empty());
}

// --- Trait object test (verifies trait is object-safe and Send+Sync) ---

struct AlwaysPassEngine;

impl ChecklistEngine for AlwaysPassEngine {
    fn load(&self, _path: &Path) -> Result<Checklist, ChecklistError> {
        Ok(make_checklist(CheckMode::DoConfirm))
    }

    fn validate(&self, checklist: &Checklist) -> Vec<CheckResult> {
        checklist
            .items
            .iter()
            .map(|item| CheckResult {
                item_id: item.id.clone(),
                status: CheckStatus::Pass,
                message: "validated".to_string(),
                timestamp: Utc::now(),
            })
            .collect()
    }

    fn execute(&self, checklist: &Checklist, mode: CheckMode) -> ExecutionReport {
        let results = self.validate(checklist);
        ExecutionReport::from_results(checklist.id.clone(), mode, results, Duration::from_millis(1))
    }
}

#[test]
fn trait_object_execute_returns_report() {
    let engine: Box<dyn ChecklistEngine> = Box::new(AlwaysPassEngine);
    let cl = make_checklist(CheckMode::ReadDo);
    let report = engine.execute(&cl, CheckMode::ReadDo);
    assert_eq!(report.passed, 2);
    assert_eq!(report.failed, 0);
    assert!(report.is_success());
}

#[test]
fn trait_object_validate_returns_results_per_item() {
    let engine: Box<dyn ChecklistEngine> = Box::new(AlwaysPassEngine);
    let cl = make_checklist(CheckMode::DoConfirm);
    let results = engine.validate(&cl);
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.status == CheckStatus::Pass));
}
