// Checklist CLI handler — exposes checklist operations for the cvg CLI.
// Why: Separates CLI glue from engine logic; each command maps to a typed method.
use crate::checklist::engine::{CheckMode, CheckResult, ExecutionReport};
use crate::checklist::registry::ChecklistRegistry;
use crate::checklist::runners::do_confirm::DoConfirmRunner;
use crate::checklist::runners::read_do::ReadDoRunner;
use serde::{Deserialize, Serialize};

/// Compact view of a checklist suitable for tabular display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChecklistSummary {
    pub name: String,
    pub version: String,
    pub mode: String,
    pub item_count: usize,
}

/// Dispatches `cvg checklist *` sub-commands.
pub struct ChecklistCliHandler;

impl ChecklistCliHandler {
    pub fn new() -> Self {
        Self
    }

    /// `cvg checklist run <name> [--mode <mode>]`
    ///
    /// Runs the named checklist.  When `mode` is `Some`, it overrides the
    /// mode declared in the checklist definition.
    pub fn run_checklist(
        &self,
        name: &str,
        mode: Option<CheckMode>,
        registry: &ChecklistRegistry,
    ) -> Result<ExecutionReport, String> {
        let checklist = registry
            .get(name)
            .ok_or_else(|| format!("checklist '{}' not found in registry", name))?;

        let effective_mode = mode.unwrap_or_else(|| checklist.mode.clone());
        let report = match effective_mode {
            CheckMode::DoConfirm => {
                // Clone with overridden mode so the runner records the right mode.
                let mut cl = checklist.clone();
                cl.mode = CheckMode::DoConfirm;
                DoConfirmRunner::new().run(&cl)
            }
            CheckMode::ReadDo => {
                let mut cl = checklist.clone();
                cl.mode = CheckMode::ReadDo;
                ReadDoRunner::new().execute(&cl)
            }
        };
        Ok(report)
    }

    /// `cvg checklist list`
    ///
    /// Returns a summary for every registered checklist.
    pub fn list_checklists(&self, registry: &ChecklistRegistry) -> Vec<ChecklistSummary> {
        let mut summaries: Vec<ChecklistSummary> = registry
            .list()
            .into_iter()
            .map(|cl| ChecklistSummary {
                name: cl.name.clone(),
                version: cl.version.clone(),
                mode: mode_label(&cl.mode),
                item_count: cl.items.len(),
            })
            .collect();
        // Stable ordering for predictable CLI output.
        summaries.sort_by(|a, b| a.name.cmp(&b.name));
        summaries
    }

    /// `cvg checklist validate <name>`
    ///
    /// Dry-runs the checklist and returns per-item results without committing
    /// side effects.  Uses the checklist's declared mode.
    pub fn validate_checklist(
        &self,
        name: &str,
        registry: &ChecklistRegistry,
    ) -> Result<Vec<CheckResult>, String> {
        let checklist = registry
            .get(name)
            .ok_or_else(|| format!("checklist '{}' not found in registry", name))?;

        let report = match checklist.mode {
            CheckMode::DoConfirm => DoConfirmRunner::new().run(checklist),
            CheckMode::ReadDo => ReadDoRunner::new().execute(checklist),
        };
        Ok(report.results)
    }
}

impl Default for ChecklistCliHandler {
    fn default() -> Self {
        Self::new()
    }
}

fn mode_label(mode: &CheckMode) -> String {
    match mode {
        CheckMode::DoConfirm => "do-confirm".to_string(),
        CheckMode::ReadDo => "read-do".to_string(),
    }
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
