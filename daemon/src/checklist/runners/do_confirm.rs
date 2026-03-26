// DO-CONFIRM runner — aviation-inspired paradigm.
// Why: all actions complete first, then operator confirms each item sequentially.
// This mirrors cockpit checklists: pilot performs all steps, co-pilot verifies.
use crate::checklist::engine::{CheckMode, CheckResult, CheckStatus, Checklist, ExecutionReport};
use chrono::Utc;
use std::process::Command;
use std::time::Instant;

/// Executes all checklist items unconditionally, then reports confirmation status.
/// A failing item never halts subsequent items — all must run before reporting.
pub struct DoConfirmRunner;

impl DoConfirmRunner {
    pub fn new() -> Self {
        Self
    }

    /// Execute all items and return a full ExecutionReport.
    /// All items run regardless of individual failures (DO-CONFIRM paradigm).
    pub fn run(&self, checklist: &Checklist) -> ExecutionReport {
        let start = Instant::now();
        let results: Vec<CheckResult> = checklist
            .items
            .iter()
            .map(|item| self.execute_item(&item.id, &item.command))
            .collect();
        let duration = start.elapsed();
        ExecutionReport::from_results(
            checklist.id.clone(),
            CheckMode::DoConfirm,
            results,
            duration,
        )
    }

    fn execute_item(&self, item_id: &str, command: &str) -> CheckResult {
        let timestamp = Utc::now();
        let outcome = Command::new("sh").arg("-c").arg(command).output();
        match outcome {
            Ok(output) if output.status.success() => CheckResult {
                item_id: item_id.to_string(),
                status: CheckStatus::Pass,
                message: format!("confirmed: {item_id}"),
                timestamp,
            },
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let code = output.status.code().unwrap_or(-1);
                CheckResult {
                    item_id: item_id.to_string(),
                    status: CheckStatus::Fail,
                    message: format!(
                        "unconfirmed: {item_id} (exit {code}{})",
                        if stderr.is_empty() {
                            String::new()
                        } else {
                            format!(", {}", stderr.trim())
                        }
                    ),
                    timestamp,
                }
            }
            Err(err) => CheckResult {
                item_id: item_id.to_string(),
                status: CheckStatus::Fail,
                message: format!("unconfirmed: {item_id} (spawn error: {err})"),
                timestamp,
            },
        }
    }
}

impl Default for DoConfirmRunner {
    fn default() -> Self {
        Self::new()
    }
}
