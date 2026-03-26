// Telemetry tests — TDD RED phase.
// Why: Validates metric accumulation, pass rate calculation, and failure tracking.
#[cfg(test)]
mod tests {
    use crate::checklist::engine::{
        CheckMode, CheckResult, CheckStatus, ExecutionReport,
    };
    use crate::checklist::telemetry::ChecklistTelemetry;
    use chrono::Utc;
    use std::time::Duration;

    fn make_report(name: &str, passed: usize, failed: usize, duration_ms: u64) -> ExecutionReport {
        let mut results = Vec::new();
        for i in 0..passed {
            results.push(CheckResult {
                item_id: format!("pass-{i}"),
                status: CheckStatus::Pass,
                message: "ok".to_string(),
                timestamp: Utc::now(),
            });
        }
        for i in 0..failed {
            results.push(CheckResult {
                item_id: format!("fail-{i}"),
                status: CheckStatus::Fail,
                message: format!("failure-{i}"),
                timestamp: Utc::now(),
            });
        }
        ExecutionReport::from_results(
            name.to_string(),
            CheckMode::DoConfirm,
            results,
            Duration::from_millis(duration_ms),
        )
    }

    #[test]
    fn record_run_increments_run_count() {
        let mut tel = ChecklistTelemetry::new();
        let report = make_report("deploy", 3, 0, 100);
        tel.record_run("deploy", &report);
        tel.record_run("deploy", &report);
        let metrics = tel.metrics_for("deploy");
        assert_eq!(metrics.run_count, 2);
    }

    #[test]
    fn pass_rate_100_when_all_pass() {
        let mut tel = ChecklistTelemetry::new();
        let report = make_report("deploy", 4, 0, 50);
        tel.record_run("deploy", &report);
        let metrics = tel.metrics_for("deploy");
        assert!((metrics.pass_rate - 1.0).abs() < f64::EPSILON, "100% pass rate expected");
    }

    #[test]
    fn pass_rate_0_when_all_fail() {
        let mut tel = ChecklistTelemetry::new();
        let report = make_report("deploy", 0, 3, 50);
        tel.record_run("deploy", &report);
        let metrics = tel.metrics_for("deploy");
        assert!(metrics.pass_rate.abs() < f64::EPSILON, "0% pass rate expected");
    }

    #[test]
    fn pass_rate_mixed() {
        let mut tel = ChecklistTelemetry::new();
        // 2 pass, 2 fail → 50%
        let report = make_report("deploy", 2, 2, 100);
        tel.record_run("deploy", &report);
        let metrics = tel.metrics_for("deploy");
        assert!((metrics.pass_rate - 0.5).abs() < f64::EPSILON, "50% pass rate expected");
    }

    #[test]
    fn avg_duration_computed_across_runs() {
        let mut tel = ChecklistTelemetry::new();
        tel.record_run("deploy", &make_report("deploy", 1, 0, 100));
        tel.record_run("deploy", &make_report("deploy", 1, 0, 200));
        let metrics = tel.metrics_for("deploy");
        assert_eq!(metrics.avg_duration_ms, 150, "average of 100 and 200 is 150ms");
    }

    #[test]
    fn common_failures_tracks_failed_item_ids() {
        let mut tel = ChecklistTelemetry::new();
        // First run: fail-0 fails
        let report1 = make_report("deploy", 1, 1, 50);
        tel.record_run("deploy", &report1);
        // Second run: fail-0 fails again
        tel.record_run("deploy", &report1);
        let metrics = tel.metrics_for("deploy");
        assert!(!metrics.common_failures.is_empty(), "failures must be tracked");
        let top = &metrics.common_failures[0];
        assert_eq!(top.1, 2, "fail-0 failed twice");
    }

    #[test]
    fn metrics_for_unknown_checklist_returns_zero_metrics() {
        let tel = ChecklistTelemetry::new();
        let metrics = tel.metrics_for("unknown");
        assert_eq!(metrics.run_count, 0);
        assert!(metrics.pass_rate.abs() < f64::EPSILON);
        assert_eq!(metrics.avg_duration_ms, 0);
        assert!(metrics.common_failures.is_empty());
    }

    #[test]
    fn all_metrics_returns_one_entry_per_tracked_checklist() {
        let mut tel = ChecklistTelemetry::new();
        tel.record_run("deploy", &make_report("deploy", 2, 0, 100));
        tel.record_run("preflight", &make_report("preflight", 1, 1, 200));
        let all = tel.all_metrics();
        assert_eq!(all.len(), 2);
        let names: Vec<&str> = all.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"deploy"));
        assert!(names.contains(&"preflight"));
    }

    #[test]
    fn multiple_checklists_tracked_independently() {
        let mut tel = ChecklistTelemetry::new();
        tel.record_run("alpha", &make_report("alpha", 3, 0, 50));
        tel.record_run("alpha", &make_report("alpha", 3, 0, 50));
        tel.record_run("beta", &make_report("beta", 0, 2, 80));
        let alpha = tel.metrics_for("alpha");
        let beta = tel.metrics_for("beta");
        assert_eq!(alpha.run_count, 2);
        assert_eq!(beta.run_count, 1);
        assert!((alpha.pass_rate - 1.0).abs() < f64::EPSILON);
        assert!(beta.pass_rate.abs() < f64::EPSILON);
    }
}
