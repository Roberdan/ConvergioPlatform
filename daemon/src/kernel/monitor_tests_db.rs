// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// DB integration tests for kernel/monitor.rs — classify_readiness_results and check_peer_readiness.

#[cfg(test)]
mod tests {
    use crate::kernel::monitor::{classify_readiness_results, check_peer_readiness};

    fn make_pool() -> r2d2::Pool<r2d2_sqlite::SqliteConnectionManager> {
        let mgr = r2d2_sqlite::SqliteConnectionManager::memory();
        let pool = r2d2::Pool::new(mgr).expect("pool");
        let conn = pool.get().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kernel_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL DEFAULT (datetime('now')),
                severity TEXT NOT NULL DEFAULT 'ok',
                source TEXT NOT NULL DEFAULT '',
                message TEXT NOT NULL DEFAULT '',
                action_taken TEXT NOT NULL DEFAULT ''
            )",
        ).unwrap();
        drop(conn);
        pool
    }

    // --- classify_readiness_results ---

    #[test]
    fn classify_readiness_critical_on_db_integrity_failure() {
        let pool = make_pool();

        // Simulate a readiness response with db_integrity failure.
        let checks = serde_json::json!([
            {"name": "db_integrity", "passed": false, "detail": "PRAGMA integrity_check failed"},
            {"name": "mlx_lm", "passed": true, "detail": "available"}
        ]);
        let critical = classify_readiness_results(&pool, "node-alpha", &checks);
        assert!(critical, "db_integrity failure must produce CRITICAL event");

        let conn = pool.get().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM kernel_events WHERE severity='critical' AND source LIKE 'readiness:%'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        assert_eq!(count, 1);
    }

    #[test]
    fn classify_readiness_warn_on_mlx_lm_failure() {
        let pool = make_pool();

        let checks = serde_json::json!([
            {"name": "mlx_lm", "passed": false, "detail": "not found"},
            {"name": "daemon_version", "passed": true, "detail": "1.0.0"}
        ]);
        let critical = classify_readiness_results(&pool, "node-beta", &checks);
        assert!(!critical, "mlx_lm failure is WARN, not CRITICAL");

        let conn = pool.get().unwrap();
        let row: (i64, String) = conn
            .query_row(
                "SELECT COUNT(*), COALESCE(MIN(severity),'') FROM kernel_events WHERE source='readiness:node-beta'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(row.0, 1, "exactly one event inserted");
        assert_eq!(row.1, "warn");
    }

    #[test]
    fn classify_readiness_critical_on_daemon_version_failure() {
        let pool = make_pool();

        let checks = serde_json::json!([
            {"name": "daemon_version", "passed": false, "detail": "mismatch: Cargo=1.0.0 VERSION.md=0.9.0"},
            {"name": "telegram", "passed": true, "detail": "ok"}
        ]);
        let critical = classify_readiness_results(&pool, "node-gamma", &checks);
        assert!(critical, "daemon_version failure must produce CRITICAL event");
    }

    #[test]
    fn classify_readiness_no_events_when_all_pass() {
        let pool = make_pool();

        let checks = serde_json::json!([
            {"name": "db_integrity", "passed": true, "detail": "ok"},
            {"name": "daemon_version", "passed": true, "detail": "1.0.0"}
        ]);
        let critical = classify_readiness_results(&pool, "node-delta", &checks);
        assert!(!critical);

        let conn = pool.get().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM kernel_events", [], |r| r.get(0))
            .unwrap_or(0);
        assert_eq!(count, 0, "no events when all checks pass");
    }

    // --- check_peer_readiness (unreachable peer) ---

    #[tokio::test]
    async fn check_peer_readiness_unreachable_fails_gracefully() {
        let pool = make_pool();

        // Port 19997 is not listening — should log a warn event, not panic.
        check_peer_readiness(&pool, "http://127.0.0.1:19997").await;

        // A warn event should have been stored for the unreachable peer.
        let conn = pool.get().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM kernel_events WHERE source LIKE 'readiness:%'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        assert_eq!(count, 1, "unreachable peer must produce a warn event");
    }
}
