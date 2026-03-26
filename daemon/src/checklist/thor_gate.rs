// Thor Gate — severity-aware checklist validation for Thor's pipeline.
// Why: Thor needs a structured gate that distinguishes Critical blockers from
// Warning advisories and Info noise before marking a plan complete.
use crate::checklist::engine::{CheckMode, CheckSeverity, CheckStatus, Checklist, ExecutionReport};
use crate::checklist::registry::ChecklistRegistry;
use crate::checklist::runners::do_confirm::DoConfirmRunner;
use crate::checklist::runners::read_do::ReadDoRunner;

/// Result produced by a single ChecklistGate validation run.
#[derive(Debug, Clone)]
pub struct ChecklistGateResult {
    /// True when no Critical items failed.
    pub passed: bool,
    /// IDs of Critical items that failed.
    pub critical_failures: Vec<String>,
    /// IDs of Warning items that failed (gate still passes).
    pub warnings: Vec<String>,
    /// Full execution report from the underlying runner.
    pub report: ExecutionReport,
}

/// Runs a checklist and maps item results to Thor gate semantics.
///
/// Severity routing:
///   Critical failure → gate blocked, added to `critical_failures`.
///   Warning failure  → gate passes, added to `warnings`.
///   Info failure     → silently ignored.
pub struct ChecklistGate;

impl ChecklistGate {
    pub fn new() -> Self {
        Self
    }

    /// Validate a single checklist according to its declared mode.
    /// Returns a `ChecklistGateResult` with severity-categorised outcomes.
    pub fn validate(&self, checklist: &Checklist) -> ChecklistGateResult {
        let report = self.run_checklist(checklist);
        self.classify(&report, checklist)
    }

    /// Validate every checklist in the registry and return one result per entry.
    pub fn validate_all(&self, registry: &ChecklistRegistry) -> Vec<ChecklistGateResult> {
        registry.list().into_iter().map(|cl| self.validate(cl)).collect()
    }

    // ── Private helpers ──────────────────────────────────────────────────────

    /// Dispatch to the correct runner based on the checklist mode.
    fn run_checklist(&self, checklist: &Checklist) -> ExecutionReport {
        match checklist.mode {
            CheckMode::DoConfirm => DoConfirmRunner::new().run(checklist),
            CheckMode::ReadDo => ReadDoRunner::new().execute(checklist),
        }
    }

    /// Map failed items to Critical/Warning buckets using the checklist item definitions.
    fn classify(&self, report: &ExecutionReport, checklist: &Checklist) -> ChecklistGateResult {
        let mut critical_failures: Vec<String> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();

        for result in &report.results {
            if result.status != CheckStatus::Fail {
                continue;
            }
            // Find the severity of the failed item from the checklist definition.
            let severity = checklist
                .items
                .iter()
                .find(|item| item.id == result.item_id)
                .map(|item| &item.severity);

            match severity {
                Some(CheckSeverity::Critical) => critical_failures.push(result.item_id.clone()),
                Some(CheckSeverity::Warning) => warnings.push(result.item_id.clone()),
                Some(CheckSeverity::Info) | None => {
                    // Info failures and unmatched items are silently ignored.
                }
            }
        }

        let passed = critical_failures.is_empty();
        ChecklistGateResult { passed, critical_failures, warnings, report: report.clone() }
    }
}

impl Default for ChecklistGate {
    fn default() -> Self {
        Self::new()
    }
}
