// Checklist Telemetry — per-checklist run metrics and failure tracking.
// Why: Ops visibility into recurring failures and run reliability without
//      an external metrics backend; data lives in-process, queried via HTTP.
use crate::checklist::engine::{CheckStatus, ExecutionReport};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Computed metrics for a single checklist, exposed via
/// `GET /api/checklists/metrics`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistMetrics {
    pub name: String,
    pub run_count: usize,
    /// Fraction of items that passed across all runs (0.0–1.0).
    pub pass_rate: f64,
    /// Mean execution duration in milliseconds across all runs.
    pub avg_duration_ms: u64,
    /// Most-failed item IDs, sorted descending by failure count.
    pub common_failures: Vec<(String, usize)>,
}

/// Raw counters accumulated per checklist name.
struct ChecklistStats {
    run_count: usize,
    total_items: usize,
    total_passed: usize,
    total_duration_ms: u64,
    /// item_id → failure count
    failure_counts: HashMap<String, usize>,
}

impl ChecklistStats {
    fn new() -> Self {
        Self {
            run_count: 0,
            total_items: 0,
            total_passed: 0,
            total_duration_ms: 0,
            failure_counts: HashMap::new(),
        }
    }

    fn absorb(&mut self, report: &ExecutionReport) {
        self.run_count += 1;
        self.total_items += report.results.len();
        self.total_passed += report.passed;
        // Duration is stored as std::time::Duration; convert to ms.
        self.total_duration_ms += report.duration.as_millis() as u64;
        for result in &report.results {
            if result.status == CheckStatus::Fail {
                *self.failure_counts.entry(result.item_id.clone()).or_insert(0) += 1;
            }
        }
    }

    fn to_metrics(&self, name: &str) -> ChecklistMetrics {
        let pass_rate = if self.total_items == 0 {
            0.0
        } else {
            self.total_passed as f64 / self.total_items as f64
        };
        let avg_duration_ms = if self.run_count == 0 {
            0
        } else {
            self.total_duration_ms / self.run_count as u64
        };
        let mut common_failures: Vec<(String, usize)> =
            self.failure_counts.iter().map(|(k, v)| (k.clone(), *v)).collect();
        // Most frequent failures first.
        common_failures.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        ChecklistMetrics {
            name: name.to_string(),
            run_count: self.run_count,
            pass_rate,
            avg_duration_ms,
            common_failures,
        }
    }
}

/// Accumulates telemetry for all checklist runs during a daemon session.
///
/// Not persisted — metrics reset on daemon restart.  For long-lived history,
/// integrate with the knowledge_base table (out of scope for this wave).
pub struct ChecklistTelemetry {
    stats: HashMap<String, ChecklistStats>,
}

impl ChecklistTelemetry {
    pub fn new() -> Self {
        Self { stats: HashMap::new() }
    }

    /// Record the results of a completed run.
    pub fn record_run(&mut self, name: &str, report: &ExecutionReport) {
        self.stats.entry(name.to_string()).or_insert_with(ChecklistStats::new).absorb(report);
    }

    /// Return computed metrics for a specific checklist.
    /// Returns zero-value metrics when the checklist has never been run.
    pub fn metrics_for(&self, name: &str) -> ChecklistMetrics {
        match self.stats.get(name) {
            Some(s) => s.to_metrics(name),
            None => ChecklistMetrics {
                name: name.to_string(),
                run_count: 0,
                pass_rate: 0.0,
                avg_duration_ms: 0,
                common_failures: vec![],
            },
        }
    }

    /// Return metrics for every tracked checklist, sorted by name.
    pub fn all_metrics(&self) -> Vec<ChecklistMetrics> {
        let mut metrics: Vec<ChecklistMetrics> =
            self.stats.keys().map(|name| self.stats[name].to_metrics(name)).collect();
        metrics.sort_by(|a, b| a.name.cmp(&b.name));
        metrics
    }
}

impl Default for ChecklistTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "telemetry_tests.rs"]
mod tests;
