// Thor gate tests — validates severity-based gate logic.
// Why: Gate must block on Critical failures, warn on Warning, ignore Info.
#[cfg(test)]
mod tests {
    use crate::checklist::engine::{
        CheckItem, CheckMode, CheckResult, CheckSeverity, CheckStatus, Checklist, ExecutionReport,
    };
    use crate::checklist::thor_gate::{ChecklistGate, ChecklistGateResult};
    use chrono::Utc;
    use std::time::Duration;

    // ── Helpers ─────────────────────────────────────────────────────────────

    fn make_item(id: &str, severity: CheckSeverity) -> CheckItem {
        CheckItem {
            id: id.to_string(),
            title: format!("Check {id}"),
            command: "true".to_string(),
            expected: String::new(),
            severity,
            depends_on: vec![],
        }
    }

    fn _pass_result(item_id: &str) -> CheckResult {
        CheckResult {
            item_id: item_id.to_string(),
            status: CheckStatus::Pass,
            message: "passed".to_string(),
            timestamp: Utc::now(),
        }
    }

    fn _fail_result(item_id: &str) -> CheckResult {
        CheckResult {
            item_id: item_id.to_string(),
            status: CheckStatus::Fail,
            message: format!("failed: {item_id}"),
            timestamp: Utc::now(),
        }
    }

    fn make_checklist(items: Vec<CheckItem>, mode: CheckMode) -> Checklist {
        Checklist {
            id: "gate-test".to_string(),
            name: "Gate Test Checklist".to_string(),
            version: "1.0.0".to_string(),
            mode,
            items,
        }
    }

    fn make_report_with_results(results: Vec<CheckResult>, mode: CheckMode) -> ExecutionReport {
        ExecutionReport::from_results("gate-test".to_string(), mode, results, Duration::from_millis(1))
    }

    // ── ChecklistGateResult tests ────────────────────────────────────────────

    #[test]
    fn gate_result_passed_when_no_critical_failures() {
        let result = ChecklistGateResult {
            passed: true,
            critical_failures: vec![],
            warnings: vec!["w1".to_string()],
            report: make_report_with_results(vec![], CheckMode::DoConfirm),
        };
        assert!(result.passed);
        assert!(result.critical_failures.is_empty());
    }

    #[test]
    fn gate_result_blocked_when_critical_failures_present() {
        let result = ChecklistGateResult {
            passed: false,
            critical_failures: vec!["critical-item".to_string()],
            warnings: vec![],
            report: make_report_with_results(vec![], CheckMode::DoConfirm),
        };
        assert!(!result.passed);
        assert_eq!(result.critical_failures.len(), 1);
    }

    // ── ChecklistGate::validate tests ───────────────────────────────────────

    #[test]
    fn validate_all_pass_returns_gate_pass() {
        // All items pass → gate passes, no failures or warnings.
        let items = vec![
            make_item("critical-ok", CheckSeverity::Critical),
            make_item("warning-ok", CheckSeverity::Warning),
        ];
        let checklist = make_checklist(items, CheckMode::DoConfirm);
        let gate = ChecklistGate::new();
        let result = gate.validate(&checklist);

        assert!(result.passed, "gate must pass when all items succeed");
        assert!(result.critical_failures.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn validate_critical_failure_blocks_gate() {
        // A Critical item that fails → gate blocked, critical_failures populated.
        let items = vec![
            make_item("step-pass", CheckSeverity::Critical),
            make_item("step-fail", CheckSeverity::Critical),
        ];
        // Override: force step-fail to be a command that always fails.
        let mut checklist = make_checklist(items, CheckMode::DoConfirm);
        checklist.items[0].command = "true".to_string();
        checklist.items[1].command = "false".to_string();

        let gate = ChecklistGate::new();
        let result = gate.validate(&checklist);

        assert!(!result.passed, "gate must fail on critical item failure");
        assert!(!result.critical_failures.is_empty(), "critical_failures must be populated");
        assert!(
            result.critical_failures.contains(&"step-fail".to_string()),
            "step-fail must appear in critical_failures"
        );
    }

    #[test]
    fn validate_warning_failure_does_not_block_gate() {
        // A Warning item that fails → gate still passes, warnings populated.
        let items = vec![make_item("warn-fail", CheckSeverity::Warning)];
        let mut checklist = make_checklist(items, CheckMode::DoConfirm);
        checklist.items[0].command = "false".to_string();

        let gate = ChecklistGate::new();
        let result = gate.validate(&checklist);

        assert!(result.passed, "gate must pass when only Warning items fail");
        assert!(result.critical_failures.is_empty(), "no critical failures");
        assert!(!result.warnings.is_empty(), "warnings must be populated");
        assert!(result.warnings.contains(&"warn-fail".to_string()));
    }

    #[test]
    fn validate_info_failure_ignored_completely() {
        // An Info item that fails → gate passes, no warnings.
        let items = vec![make_item("info-fail", CheckSeverity::Info)];
        let mut checklist = make_checklist(items, CheckMode::DoConfirm);
        checklist.items[0].command = "false".to_string();

        let gate = ChecklistGate::new();
        let result = gate.validate(&checklist);

        assert!(result.passed, "gate must pass when only Info items fail");
        assert!(result.critical_failures.is_empty());
        assert!(result.warnings.is_empty(), "Info failures must not populate warnings");
    }

    #[test]
    fn validate_mixed_severity_uses_correct_routing() {
        // Critical pass + Warning fail + Info fail → gate passes with warning only.
        let items = vec![
            make_item("crit-pass", CheckSeverity::Critical),
            make_item("warn-fail", CheckSeverity::Warning),
            make_item("info-fail", CheckSeverity::Info),
        ];
        let mut checklist = make_checklist(items, CheckMode::DoConfirm);
        checklist.items[0].command = "true".to_string();
        checklist.items[1].command = "false".to_string();
        checklist.items[2].command = "false".to_string();

        let gate = ChecklistGate::new();
        let result = gate.validate(&checklist);

        assert!(result.passed);
        assert!(result.critical_failures.is_empty());
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0], "warn-fail");
    }

    #[test]
    fn validate_empty_checklist_passes() {
        // An empty checklist has no failures → gate passes trivially.
        let checklist = make_checklist(vec![], CheckMode::ReadDo);
        let gate = ChecklistGate::new();
        let result = gate.validate(&checklist);

        assert!(result.passed);
        assert!(result.critical_failures.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn validate_read_do_mode_also_supported() {
        // validate() works for both DoConfirm and ReadDo modes.
        let items = vec![make_item("s1", CheckSeverity::Critical)];
        let mut checklist = make_checklist(items, CheckMode::ReadDo);
        checklist.items[0].command = "echo ok".to_string();
        checklist.items[0].expected = "ok".to_string();

        let gate = ChecklistGate::new();
        let result = gate.validate(&checklist);

        assert!(result.passed);
    }

    #[test]
    fn validate_report_is_populated() {
        // The returned report must carry the full execution results.
        let items = vec![make_item("s1", CheckSeverity::Info)];
        let checklist = make_checklist(items, CheckMode::DoConfirm);
        let gate = ChecklistGate::new();
        let result = gate.validate(&checklist);

        assert_eq!(result.report.checklist_id, "gate-test");
        assert_eq!(result.report.results.len(), 1);
    }

}

#[cfg(test)]
#[path = "thor_gate_validate_all_tests.rs"]
mod validate_all_tests;
