// Checklist Engine — aviation-inspired DO-CONFIRM and READ-DO paradigms.
// Why: Codified runbooks eliminate human error in critical ops sequences.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

/// Execution paradigm for the checklist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckMode {
    /// All actions are performed first, then operator confirms each item.
    DoConfirm,
    /// Items are presented, executed, and verified strictly in sequence.
    ReadDo,
}

/// Outcome of a single checklist item execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckStatus {
    Pass,
    Fail,
    Skip,
}

/// Severity level for a checklist item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckSeverity {
    Critical,
    Warning,
    Info,
}

/// A single verifiable step in a checklist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckItem {
    pub id: String,
    pub title: String,
    /// Shell command to execute.
    pub command: String,
    /// Expected stdout content for pass determination.
    pub expected: String,
    pub severity: CheckSeverity,
    /// IDs of items that must pass before this one runs.
    pub depends_on: Vec<String>,
}

/// A named, versioned collection of checklist items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checklist {
    pub id: String,
    pub name: String,
    pub version: String,
    pub mode: CheckMode,
    pub items: Vec<CheckItem>,
}

/// Result for one checklist item after execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub item_id: String,
    pub status: CheckStatus,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// Aggregate report for a full checklist run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub checklist_id: String,
    pub mode: CheckMode,
    pub results: Vec<CheckResult>,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration: Duration,
}

impl ExecutionReport {
    /// Derive counts from results slice.
    pub fn from_results(
        checklist_id: String,
        mode: CheckMode,
        results: Vec<CheckResult>,
        duration: Duration,
    ) -> Self {
        let passed = results.iter().filter(|r| r.status == CheckStatus::Pass).count();
        let failed = results.iter().filter(|r| r.status == CheckStatus::Fail).count();
        let skipped = results.iter().filter(|r| r.status == CheckStatus::Skip).count();
        Self { checklist_id, mode, results, passed, failed, skipped, duration }
    }

    /// True when no critical items failed.
    pub fn is_success(&self) -> bool {
        self.failed == 0
    }
}

/// Core interface every checklist runner must satisfy.
pub trait ChecklistEngine: Send + Sync {
    /// Load a checklist definition from a file path.
    fn load(&self, path: &Path) -> Result<Checklist, ChecklistError>;

    /// Validate a checklist definition without executing it.
    fn validate(&self, checklist: &Checklist) -> Vec<CheckResult>;

    /// Execute all items according to the checklist mode.
    fn execute(&self, checklist: &Checklist, mode: CheckMode) -> ExecutionReport;
}

/// Errors produced by checklist operations.
#[derive(Debug, thiserror::Error)]
pub enum ChecklistError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Item '{0}' not found")]
    ItemNotFound(String),
    #[error("Circular dependency involving item '{0}'")]
    CircularDependency(String),
}


