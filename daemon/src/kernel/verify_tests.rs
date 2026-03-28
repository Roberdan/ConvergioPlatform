// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for kernel::verify — evidence gate, EvidenceCheck/Report, DB persistence.

#[cfg(test)]
mod tests {
    use crate::kernel::verify::{check_evidence, EvidenceCheck, EvidenceReport};
    use crate::kernel::verify_checks::build_situation_string;
    use crate::kernel::engine::{KernelConfig, KernelEngine};
    use rusqlite::Connection;
    use std::path::Path;

    fn make_engine() -> KernelEngine {
        KernelEngine::new(KernelConfig::default())
    }

    fn make_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kernel_verifications (
                id             INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id        INTEGER,
                timestamp      TEXT NOT NULL DEFAULT (datetime('now')),
                checks_json    TEXT NOT NULL DEFAULT '[]',
                passed         INTEGER NOT NULL DEFAULT 1,
                blocked_reason TEXT
            );",
        )
        .expect("schema");
        conn
    }

    // --- EvidenceCheck construction ---

    #[test]
    fn evidence_check_pass_sets_passed_true() {
        let c = EvidenceCheck::pass("my_check", "all good");
        assert!(c.passed);
        assert_eq!(c.name, "my_check");
        assert_eq!(c.detail, "all good");
    }

    #[test]
    fn evidence_check_fail_sets_passed_false() {
        let c = EvidenceCheck::fail("my_check", "missing file");
        assert!(!c.passed);
    }

    // --- EvidenceReport helpers ---

    #[test]
    fn evidence_report_failed_checks_filters_correctly() {
        let checks = vec![
            EvidenceCheck::pass("a", "ok"),
            EvidenceCheck::fail("b", "bad"),
            EvidenceCheck::fail("c", "also bad"),
        ];
        let report = EvidenceReport {
            task_id: 1,
            status_requested: "done".to_string(),
            passed: false,
            checks,
            severity: "warn".to_string(),
            action: "alert".to_string(),
            reason: "failures".to_string(),
        };
        let failed = report.failed_checks();
        assert_eq!(failed.len(), 2);
        assert!(failed.iter().any(|c| c.name == "b"));
        assert!(failed.iter().any(|c| c.name == "c"));
    }

    // --- File existence check (inline) ---

    #[test]
    fn output_file_check_passes_for_existing_file() {
        // Use this very source file as the existing file.
        let path = file!();
        assert!(Path::new(path).exists() || {
            // file! gives a relative path; try absolute fallback.
            let abs = format!(
                "{}/{}",
                env!("CARGO_MANIFEST_DIR"),
                path
            );
            Path::new(&abs).exists()
        });
    }

    #[test]
    fn output_file_check_fails_for_missing_file() {
        let c = if Path::new("/nonexistent/file/kernel_verify_test.rs").exists() {
            EvidenceCheck::pass("output_file_exists", "unexpected")
        } else {
            EvidenceCheck::fail(
                "output_file_exists",
                "file not found: /nonexistent/file/kernel_verify_test.rs",
            )
        };
        assert!(!c.passed);
    }

    // --- DB persistence ---

    #[test]
    fn check_evidence_persists_record_to_db() {
        let conn = make_conn();
        let engine = make_engine();

        // Pass no output_files; cargo/git checks will run — may pass or fail
        // in CI. We only assert the record is written.
        let report = check_evidence(&conn, &engine, 42, "done", None, &[]);

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM kernel_verifications WHERE task_id = 42",
                [],
                |r| r.get(0),
            )
            .expect("count query");
        assert_eq!(count, 1, "verification record must be written to DB");
        assert_eq!(report.task_id, 42);
        assert_eq!(report.status_requested, "done");
    }

    #[test]
    fn check_evidence_passed_field_matches_db_record() {
        let conn = make_conn();
        let engine = make_engine();

        let report = check_evidence(&conn, &engine, 99, "submitted", None, &[]);

        let db_passed: i64 = conn
            .query_row(
                "SELECT passed FROM kernel_verifications WHERE task_id = 99",
                [],
                |r| r.get(0),
            )
            .expect("passed query");

        let expected = if report.passed { 1i64 } else { 0i64 };
        assert_eq!(db_passed, expected);
    }

    #[test]
    fn check_evidence_blocked_reason_null_when_passed() {
        // Simulate a scenario with no output files; other checks may vary.
        // We only care about the blocked_reason column when passed = true.
        let conn = make_conn();
        let engine = make_engine();
        let report = check_evidence(&conn, &engine, 7, "done", None, &[]);

        if report.passed {
            let blocked: Option<String> = conn
                .query_row(
                    "SELECT blocked_reason FROM kernel_verifications WHERE task_id = 7",
                    [],
                    |r| r.get(0),
                )
                .expect("blocked query");
            assert!(blocked.is_none(), "blocked_reason must be NULL when passed");
        }
        // If not passed, blocked_reason is set — valid either way.
    }

    // --- KernelCheckResult interop ---

    #[test]
    fn from_kernel_check_result_pass() {
        use crate::kernel::monitor::KernelCheckResult;
        let kcr = KernelCheckResult::pass("daemon_health");
        let ec = EvidenceCheck::from(kcr);
        assert!(ec.passed);
        assert_eq!(ec.name, "daemon_health");
    }

    #[test]
    fn from_kernel_check_result_fail() {
        use crate::kernel::monitor::KernelCheckResult;
        let kcr = KernelCheckResult::fail("daemon_health", "HTTP 503");
        let ec = EvidenceCheck::from(kcr);
        assert!(!ec.passed);
        assert_eq!(ec.detail, "HTTP 503");
    }

    // --- Build situation string ---

    #[test]
    fn situation_string_all_pass() {
        let checks = vec![EvidenceCheck::pass("a", "ok"), EvidenceCheck::pass("b", "ok")];
        let s = build_situation_string(&checks);
        assert!(s.contains("passed"), "situation: {s}");
    }

    #[test]
    fn situation_string_with_failures() {
        let checks = vec![
            EvidenceCheck::pass("a", "ok"),
            EvidenceCheck::fail("cargo_check", "error"),
        ];
        let s = build_situation_string(&checks);
        assert!(s.contains("cargo_check"), "situation: {s}");
    }

    // --- Serialization ---

    #[test]
    fn evidence_report_serializes_to_json() {
        let report = EvidenceReport {
            task_id: 5,
            status_requested: "done".to_string(),
            passed: false,
            checks: vec![EvidenceCheck::fail("cargo_test", "1 failed")],
            severity: "critical".to_string(),
            action: "block".to_string(),
            reason: "tests failed".to_string(),
        };
        let json = serde_json::to_string(&report).expect("serialize");
        assert!(json.contains("cargo_test"));
        assert!(json.contains("task_id"));
        assert!(json.contains("kernel_verifications") || json.contains("checks"));
    }

}
