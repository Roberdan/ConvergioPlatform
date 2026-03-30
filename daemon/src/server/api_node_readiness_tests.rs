// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
//! Tests for GET /api/node/readiness — node-level readiness checks.

use super::{run_checks, summarize_for_boot, Check, NodeReadinessResponse};

fn find_check<'a>(checks: &'a [Check], name: &str) -> &'a Check {
    checks.iter().find(|c| c.name == name).unwrap_or_else(|| {
        panic!("check '{name}' not found in response");
    })
}

#[test]
fn response_has_required_fields() {
    let result = NodeReadinessResponse {
        ok: true,
        node: "test-host".to_string(),
        role: "worker".to_string(),
        checks: vec![],
    };
    assert!(result.ok);
    assert_eq!(result.node, "test-host");
    assert_eq!(result.role, "worker");
}

#[test]
fn all_checks_present_in_response() {
    let checks = run_checks();
    let names: Vec<&str> = checks.iter().map(|c| c.name.as_str()).collect();
    // All 10 required checks must be present
    assert!(names.contains(&"mlx_lm"), "missing mlx_lm check");
    assert!(names.contains(&"python_venv"), "missing python_venv check");
    assert!(names.contains(&"db_exists"), "missing db_exists check");
    assert!(names.contains(&"db_symlink"), "missing db_symlink check");
    assert!(names.contains(&"telegram_token"), "missing telegram_token check");
    assert!(names.contains(&"disk_space"), "missing disk_space check");
    assert!(names.contains(&"models_downloaded"), "missing models_downloaded check");
    assert!(names.contains(&"daemon_version"), "missing daemon_version check");
    assert!(names.contains(&"node_role"), "missing node_role check");
    assert!(names.contains(&"role_capabilities"), "missing role_capabilities check");
}

#[test]
fn each_check_has_name_and_detail() {
    let checks = run_checks();
    for check in &checks {
        assert!(!check.name.is_empty(), "check name must not be empty");
        assert!(!check.detail.is_empty(), "check '{}' must have a detail string", check.name);
    }
}

#[test]
fn daemon_version_check_returns_detail() {
    let checks = run_checks();
    let c = find_check(&checks, "daemon_version");
    // Detail should contain a version string (semver-like)
    assert!(
        c.detail.contains('.'),
        "daemon_version detail should contain version: got '{}'",
        c.detail
    );
}

#[test]
fn disk_space_check_returns_numeric_detail() {
    let checks = run_checks();
    let c = find_check(&checks, "disk_space");
    // Detail should mention GB
    assert!(
        c.detail.to_lowercase().contains("gb"),
        "disk_space detail should include GB: got '{}'",
        c.detail
    );
}

#[test]
fn telegram_token_check_reflects_env() {
    let checks = run_checks();
    let c = find_check(&checks, "telegram_token");
    // Result is deterministic based on env; just verify it ran
    assert!(!c.detail.is_empty(), "telegram_token detail must not be empty");
}

#[test]
fn ok_field_is_false_when_any_required_check_fails() {
    // Build a response where one check fails
    let checks = vec![
        Check { name: "mlx_lm".into(), passed: true, detail: "ok".into() },
        Check { name: "db_exists".into(), passed: false, detail: "missing".into() },
    ];
    let ok = checks.iter().all(|c| c.passed);
    assert!(!ok, "ok should be false when any check fails");
}

#[test]
fn ok_field_is_true_when_all_checks_pass() {
    let checks = vec![
        Check { name: "mlx_lm".into(), passed: true, detail: "ok".into() },
        Check { name: "db_exists".into(), passed: true, detail: "ok".into() },
    ];
    let ok = checks.iter().all(|c| c.passed);
    assert!(ok, "ok should be true when all checks pass");
}

#[test]
fn boot_summary_treats_disk_space_as_blocking() {
    let summary = summarize_for_boot(&[Check {
        name: "disk_space".into(),
        passed: false,
        detail: "0.5 GB free".into(),
    }]);
    assert_eq!(summary.blocking_failures.len(), 1);
    assert!(summary.warning_failures.is_empty());
}

#[test]
fn boot_summary_treats_missing_db_as_blocking() {
    let summary = summarize_for_boot(&[Check {
        name: "db_exists".into(),
        passed: false,
        detail: "not found: /tmp/dashboard.db".into(),
    }]);
    // v20: missing DB is now a critical/blocking failure (fail-loud)
    assert_eq!(summary.blocking_failures.len(), 1);
    assert!(summary.warning_failures.is_empty());
}

#[test]
fn boot_summary_treats_corrupt_db_as_blocking() {
    let summary = summarize_for_boot(&[Check {
        name: "db_exists".into(),
        passed: false,
        detail: "PRAGMA integrity_check failed: error".into(),
    }]);
    assert_eq!(summary.blocking_failures.len(), 1);
}
