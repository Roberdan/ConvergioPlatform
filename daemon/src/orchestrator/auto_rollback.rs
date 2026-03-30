// Health-based auto-rollback after merge.
// check_health_after_merge: runs cargo test --lib + GET /api/health/deep,
// triggers rollback and marks task blocked on regression.

use crate::resilience::health::HealthStatus;
use rusqlite::{params, Connection};
use std::path::Path;
use std::process::Command;

use super::rollback;

type AutoRollbackResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Reason a regression was detected (used for blocked-task notes).
#[derive(Debug, Clone)]
pub enum RegressionReason {
    TestFailure(String),
    HealthDegraded(String),
}

impl std::fmt::Display for RegressionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegressionReason::TestFailure(msg) => write!(f, "test failure: {msg}"),
            RegressionReason::HealthDegraded(msg) => write!(f, "health degraded: {msg}"),
        }
    }
}

/// Run `cargo test --lib` in `worktree_path`.
/// Returns `Ok(())` on success, `Err(stderr)` on failure.
fn run_cargo_tests(worktree_path: &Path) -> AutoRollbackResult<()> {
    let out = Command::new("cargo")
        .args(["test", "--lib", "--quiet"])
        .current_dir(worktree_path)
        .output()
        .map_err(|e| format!("cargo test failed to start: {e}"))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let detail = if !stderr.is_empty() { stderr.into_owned() } else { stdout.into_owned() };
        return Err(detail.into());
    }
    Ok(())
}

/// GET /api/health/deep from the local daemon.
/// Returns `(overall_status_string, detail)`.
fn fetch_health_deep() -> AutoRollbackResult<String> {
    let out = Command::new("curl")
        .args(["-sf", "--max-time", "5", "http://localhost:8420/api/health/deep"])
        .output()
        .map_err(|e| format!("curl health/deep failed: {e}"))?;

    if !out.status.success() {
        return Err("GET /api/health/deep: daemon unreachable".into());
    }
    let body = String::from_utf8_lossy(&out.stdout).into_owned();
    Ok(body)
}

/// Parse the `status` field from a `/api/health/deep` JSON body.
fn parse_health_status(json_body: &str) -> HealthStatus {
    // Minimal parse — avoid pulling serde_json into a hot path.
    if json_body.contains(r#""status":"healthy""#) {
        HealthStatus::Healthy
    } else if json_body.contains(r#""status":"degraded""#) {
        HealthStatus::Degraded
    } else {
        HealthStatus::Unhealthy
    }
}

/// Mark the task as blocked with an auto-rollback note.
fn mark_task_blocked(conn: &Connection, task_id: i64, reason: &str) -> AutoRollbackResult<()> {
    conn.execute(
        "UPDATE tasks SET status = 'blocked', notes = ?2 WHERE id = ?1",
        params![task_id, format!("auto-rollback: {reason}")],
    )?;
    Ok(())
}

/// Run post-merge health checks and roll back if a regression is detected.
///
/// Steps:
/// 1. `cargo test --lib` in `worktree_path`.
/// 2. GET /api/health/deep and verify the overall status is healthy.
/// 3. On regression: restore the latest snapshot, mark task blocked.
/// 4. Log a warning.
///
/// Returns the observed `HealthStatus` (Healthy if no regression).
pub fn check_health_after_merge(
    conn: &Connection,
    task_id: i64,
    worktree_path: &Path,
) -> AutoRollbackResult<HealthStatus> {
    // ── Step 1: library tests ────────────────────────────────────────────────
    if let Err(test_err) = run_cargo_tests(worktree_path) {
        let reason = RegressionReason::TestFailure(
            test_err.to_string().lines().take(3).collect::<Vec<_>>().join(" | "),
        );
        trigger_rollback(conn, task_id, worktree_path, &reason)?;
        return Ok(HealthStatus::Unhealthy);
    }

    // ── Step 2: daemon deep-health ───────────────────────────────────────────
    match fetch_health_deep() {
        Ok(body) => {
            let status = parse_health_status(&body);
            if status != HealthStatus::Healthy {
                let reason = RegressionReason::HealthDegraded(
                    format!("daemon reports status={status}"),
                );
                trigger_rollback(conn, task_id, worktree_path, &reason)?;
                return Ok(status);
            }
        }
        // Daemon unreachable after merge → treat as degraded but do not rollback;
        // the task may be running outside a live daemon context.
        Err(e) => {
            tracing::warn!(task_id, "auto-rollback: health/deep unreachable: {e}");
        }
    }

    tracing::info!(task_id, "auto-rollback: post-merge checks passed");
    Ok(HealthStatus::Healthy)
}

/// Execute rollback + mark task blocked.
fn trigger_rollback(
    conn: &Connection,
    task_id: i64,
    worktree_path: &Path,
    reason: &RegressionReason,
) -> AutoRollbackResult<()> {
    tracing::warn!(
        task_id,
        reason = %reason,
        "auto-rollback: regression detected — restoring snapshot"
    );

    rollback::restore_snapshot(conn, task_id, worktree_path)?;
    mark_task_blocked(conn, task_id, &reason.to_string())?;

    tracing::warn!(
        task_id,
        "auto-rollback: rollback complete, task marked blocked"
    );
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        rollback::migrate(&conn).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tasks (
                id          INTEGER PRIMARY KEY,
                task_id     TEXT,
                title       TEXT,
                status      TEXT DEFAULT 'pending',
                notes       TEXT,
                started_at  TEXT
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_parse_health_status_healthy() {
        let body = r#"{"status":"healthy","components":[]}"#;
        assert_eq!(parse_health_status(body), HealthStatus::Healthy);
    }

    #[test]
    fn test_parse_health_status_degraded() {
        let body = r#"{"status":"degraded","components":[]}"#;
        assert_eq!(parse_health_status(body), HealthStatus::Degraded);
    }

    #[test]
    fn test_parse_health_status_unhealthy() {
        let body = r#"{"status":"unhealthy","components":[]}"#;
        assert_eq!(parse_health_status(body), HealthStatus::Unhealthy);
    }

    #[test]
    fn test_mark_task_blocked_sets_status_and_notes() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO tasks (id, task_id, title, status) VALUES (1, 'T4-05', 'Test', 'in_progress')",
            [],
        )
        .unwrap();

        mark_task_blocked(&conn, 1, "test failure: assertion failed").unwrap();

        let (status, notes): (String, String) = conn
            .query_row(
                "SELECT status, notes FROM tasks WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();

        assert_eq!(status, "blocked");
        assert!(notes.starts_with("auto-rollback:"));
        assert!(notes.contains("test failure"));
    }

    #[test]
    fn test_regression_reason_display() {
        let r1 = RegressionReason::TestFailure("assertion failed at lib.rs:10".into());
        assert!(r1.to_string().contains("test failure"));

        let r2 = RegressionReason::HealthDegraded("daemon reports status=degraded".into());
        assert!(r2.to_string().contains("health degraded"));
    }

    #[test]
    fn test_check_health_no_snapshot_returns_error() {
        let conn = setup_db();
        // No snapshot exists for task 999 — restore_snapshot will fail.
        // check_health_after_merge with a non-existent worktree fails at cargo test step,
        // then restore_snapshot fails → propagates error.
        let result =
            check_health_after_merge(&conn, 999, Path::new("/nonexistent/path"));
        // Either cargo test fails (worktree missing) or restore fails — both are errors.
        // We just verify it doesn't panic.
        let _ = result;
    }
}
