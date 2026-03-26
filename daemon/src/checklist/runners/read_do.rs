// READ-DO Runner — strict sequential execution.
// Why: Aviation READ-DO paradigm requires operator to read each item,
// execute the action, then verify before proceeding to the next.
use crate::checklist::engine::{
    CheckItem, CheckMode, CheckResult, CheckSeverity, CheckStatus, Checklist, ExecutionReport,
};
use chrono::Utc;
use std::process::Command;
use std::time::Instant;

/// Executes checklist items strictly in order.
/// Critical failures halt execution; Warning failures log and continue.
pub struct ReadDoRunner;

impl ReadDoRunner {
    pub fn new() -> Self {
        Self
    }

    /// Execute a single item: run command, check exit code and expected output.
    fn run_item(&self, item: &CheckItem) -> CheckResult {
        let result = Command::new("sh").args(["-c", &item.command]).output();

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let passed = output.status.success()
                    && (item.expected.is_empty()
                        || stdout.trim().contains(item.expected.trim()));
                CheckResult {
                    item_id: item.id.clone(),
                    status: if passed { CheckStatus::Pass } else { CheckStatus::Fail },
                    message: if passed {
                        stdout.trim().to_string()
                    } else {
                        format!(
                            "exit={}, stdout={}, expected={}",
                            output.status.code().unwrap_or(-1),
                            stdout.trim(),
                            item.expected
                        )
                    },
                    timestamp: Utc::now(),
                }
            }
            Err(e) => CheckResult {
                item_id: item.id.clone(),
                status: CheckStatus::Fail,
                message: format!("command error: {e}"),
                timestamp: Utc::now(),
            },
        }
    }

    /// Run all items in declaration order, halting on Critical failure.
    pub fn execute(&self, checklist: &Checklist) -> ExecutionReport {
        let start = Instant::now();
        let mut results: Vec<CheckResult> = Vec::new();

        for item in &checklist.items {
            let result = self.run_item(item);
            let failed = result.status == CheckStatus::Fail;

            results.push(result);

            if failed && item.severity == CheckSeverity::Critical {
                // Remaining items are skipped after Critical halt.
                for remaining in checklist.items.iter().skip(results.len()) {
                    results.push(CheckResult {
                        item_id: remaining.id.clone(),
                        status: CheckStatus::Skip,
                        message: "halted by critical failure".to_string(),
                        timestamp: Utc::now(),
                    });
                }
                break;
            }
            // Warning severity: log in message, continue.
        }

        ExecutionReport::from_results(
            checklist.id.clone(),
            CheckMode::ReadDo,
            results,
            start.elapsed(),
        )
    }
}

impl Default for ReadDoRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checklist::engine::CheckSeverity;

    fn make_item(id: &str, cmd: &str, expected: &str, severity: CheckSeverity) -> CheckItem {
        CheckItem {
            id: id.to_string(),
            title: format!("Check {id}"),
            command: cmd.to_string(),
            expected: expected.to_string(),
            severity,
            depends_on: vec![],
        }
    }

    fn make_checklist(items: Vec<CheckItem>) -> Checklist {
        Checklist {
            id: "test-cl".to_string(),
            name: "Test Checklist".to_string(),
            version: "1.0.0".to_string(),
            mode: CheckMode::ReadDo,
            items,
        }
    }

    #[test]
    fn sequential_order_enforced() {
        // Items execute and results appear in declaration order.
        let items = vec![
            make_item("step-1", "echo alpha", "alpha", CheckSeverity::Info),
            make_item("step-2", "echo beta", "beta", CheckSeverity::Info),
            make_item("step-3", "echo gamma", "gamma", CheckSeverity::Info),
        ];
        let cl = make_checklist(items);
        let runner = ReadDoRunner::new();
        let report = runner.execute(&cl);

        assert_eq!(report.results.len(), 3, "all items must produce a result");
        assert_eq!(report.results[0].item_id, "step-1");
        assert_eq!(report.results[1].item_id, "step-2");
        assert_eq!(report.results[2].item_id, "step-3");
        assert_eq!(report.passed, 3);
        assert_eq!(report.failed, 0);
    }

    #[test]
    fn critical_failure_halts_remaining_items() {
        // A Critical item that fails must stop further execution.
        // Remaining items are recorded as Skip with a halt message.
        let items = vec![
            make_item("step-1", "echo ok", "ok", CheckSeverity::Info),
            make_item("step-2", "exit 1", "", CheckSeverity::Critical),
            make_item("step-3", "echo should-not-run", "should-not-run", CheckSeverity::Info),
        ];
        let cl = make_checklist(items);
        let runner = ReadDoRunner::new();
        let report = runner.execute(&cl);

        assert_eq!(report.results.len(), 3, "all items must appear in report");
        assert_eq!(report.results[0].status, CheckStatus::Pass, "step-1 passes");
        assert_eq!(report.results[1].status, CheckStatus::Fail, "step-2 fails critically");
        assert_eq!(report.results[2].status, CheckStatus::Skip, "step-3 skipped after halt");
        assert!(
            report.results[2].message.contains("halted"),
            "skip message must indicate halt"
        );
        assert_eq!(report.failed, 1);
        assert_eq!(report.skipped, 1);
        assert!(!report.is_success(), "report is failure when critical item failed");
    }

    #[test]
    fn warning_failure_continues_execution() {
        // A Warning item that fails must not halt; next items still execute.
        let items = vec![
            make_item("step-1", "exit 1", "", CheckSeverity::Warning),
            make_item("step-2", "echo continue", "continue", CheckSeverity::Info),
        ];
        let cl = make_checklist(items);
        let runner = ReadDoRunner::new();
        let report = runner.execute(&cl);

        assert_eq!(report.results.len(), 2);
        assert_eq!(report.results[0].status, CheckStatus::Fail, "step-1 fails");
        assert_eq!(report.results[1].status, CheckStatus::Pass, "step-2 still runs");
        assert_eq!(report.failed, 1);
        assert_eq!(report.passed, 1);
        assert_eq!(report.skipped, 0);
    }

    #[test]
    fn report_mode_is_read_do() {
        let cl = make_checklist(vec![make_item("s1", "echo x", "x", CheckSeverity::Info)]);
        let runner = ReadDoRunner::new();
        let report = runner.execute(&cl);
        assert_eq!(report.mode, CheckMode::ReadDo);
    }

    #[test]
    fn all_pass_report_is_success() {
        let items = vec![
            make_item("s1", "echo pass", "pass", CheckSeverity::Critical),
            make_item("s2", "echo pass", "pass", CheckSeverity::Warning),
        ];
        let cl = make_checklist(items);
        let runner = ReadDoRunner::new();
        let report = runner.execute(&cl);
        assert!(report.is_success());
        assert_eq!(report.passed, 2);
        assert_eq!(report.failed, 0);
        assert_eq!(report.skipped, 0);
    }
}
