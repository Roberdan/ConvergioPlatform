// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// TDD tests for kernel/monitor.rs.

#[cfg(test)]
mod tests {
    use crate::kernel::monitor::{
        check_daemon_reachable, check_mesh_peers, check_disk_ram, classify_and_store,
        detect_compaction_risk, detect_stale_locks, peer_name_from_url,
        classify_readiness_results, KernelCheckResult, MonitorConfig,
    };

    // --- KernelCheckResult construction ---

    #[test]
    fn kernel_check_result_pass() {
        let r = KernelCheckResult::pass("daemon_health");
        assert!(r.ok);
        assert_eq!(r.check_name, "daemon_health");
        assert!(r.details.is_none());
    }

    #[test]
    fn kernel_check_result_fail_carries_details() {
        let r = KernelCheckResult::fail("stale_locks", "3 stale locks found");
        assert!(!r.ok);
        assert_eq!(r.check_name, "stale_locks");
        assert_eq!(r.details.as_deref(), Some("3 stale locks found"));
    }

    // --- detect_stale_locks ---

    #[test]
    fn detect_stale_locks_returns_check_result() {
        let result = detect_stale_locks(300);
        assert!(!result.check_name.is_empty());
    }

    // --- check_daemon_reachable ---

    #[tokio::test]
    async fn check_daemon_reachable_unreachable_host_fails_gracefully() {
        let result = check_daemon_reachable("http://127.0.0.1:19999").await;
        assert!(!result.ok);
        assert!(result.details.is_some());
    }

    // --- check_mesh_peers ---

    #[tokio::test]
    async fn check_mesh_peers_empty_returns_empty() {
        let results = check_mesh_peers(&[]).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn check_mesh_peers_unreachable_peer_fails() {
        let peers = vec!["http://127.0.0.1:19998".to_string()];
        let results = check_mesh_peers(&peers).await;
        assert_eq!(results.len(), 1);
        assert!(!results[0].ok);
        assert!(results[0].check_name.contains("peer_health:"));
    }

    // --- detect_compaction_risk ---

    #[test]
    fn compaction_risk_below_threshold_passes() {
        let r = detect_compaction_risk(100_000, 180_000);
        assert!(r.ok, "55% usage should be OK");
    }

    #[test]
    fn compaction_risk_above_threshold_fails() {
        let r = detect_compaction_risk(160_000, 180_000); // ~88.9%
        assert!(!r.ok, "88.9% usage should trigger WARN");
        assert!(r.details.is_some());
    }

    #[test]
    fn compaction_risk_zero_limit_always_passes() {
        let r = detect_compaction_risk(999_999, 0);
        assert!(r.ok);
    }

    // --- check_disk_ram ---

    #[test]
    fn check_disk_ram_returns_nonempty_results() {
        let results = check_disk_ram();
        // Must return at least one result (RAM pressure check).
        assert!(!results.is_empty());
        for r in &results {
            assert!(!r.check_name.is_empty());
        }
    }

    // --- MonitorConfig ---

    #[test]
    fn monitor_config_default_is_local_daemon() {
        let cfg = MonitorConfig::default();
        assert!(cfg.daemon_url.contains("127.0.0.1"));
        assert!(cfg.peer_urls.is_empty());
        assert!(cfg.compaction_token_limit > 0);
    }

    // --- classify_and_store (in-memory pool) ---

    #[test]
    fn classify_and_store_critical_on_daemon_health_failure() {
        use r2d2::Pool;
        use r2d2_sqlite::SqliteConnectionManager;

        let mgr = SqliteConnectionManager::memory();
        let pool = Pool::new(mgr).expect("pool");
        // Ensure table exists.
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

        let results = vec![KernelCheckResult::fail("daemon_health", "connection refused")];
        let critical = classify_and_store(&pool, &results);
        assert!(critical, "daemon_health failure must be classified as CRITICAL");

        // Verify row persisted.
        let conn = pool.get().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM kernel_events WHERE severity='critical'", [], |r| r.get(0))
            .unwrap_or(0);
        assert_eq!(count, 1);
    }

    #[test]
    fn classify_and_store_warn_for_non_peer_check() {
        use r2d2::Pool;
        use r2d2_sqlite::SqliteConnectionManager;

        let mgr = SqliteConnectionManager::memory();
        let pool = Pool::new(mgr).expect("pool");
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

        let results = vec![KernelCheckResult::fail("ram_pressure", "85% RAM used")];
        let critical = classify_and_store(&pool, &results);
        assert!(!critical, "ram_pressure should be WARN, not CRITICAL");
    }

    // --- peer_name_from_url ---

    #[test]
    fn peer_name_from_url_extracts_hostname() {
        assert_eq!(peer_name_from_url("http://mac-worker-2:8420"), "mac-worker-2");
        assert_eq!(peer_name_from_url("http://192.168.1.10:8420"), "192.168.1.10");
        assert_eq!(peer_name_from_url("http://127.0.0.1:8420"), "127.0.0.1");
    }

    #[test]
    fn peer_name_from_url_fallback_for_bare_url() {
        // No port in URL — should still return the host portion.
        assert_eq!(peer_name_from_url("http://mynode/"), "mynode");
    }

    // --- classify_readiness_results ---

    #[test]
    fn classify_readiness_critical_on_db_integrity_failure() {
        use r2d2::Pool;
        use r2d2_sqlite::SqliteConnectionManager;

        let mgr = SqliteConnectionManager::memory();
        let pool = Pool::new(mgr).expect("pool");
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
        use r2d2::Pool;
        use r2d2_sqlite::SqliteConnectionManager;

        let mgr = SqliteConnectionManager::memory();
        let pool = Pool::new(mgr).expect("pool");
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
        use r2d2::Pool;
        use r2d2_sqlite::SqliteConnectionManager;

        let mgr = SqliteConnectionManager::memory();
        let pool = Pool::new(mgr).expect("pool");
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

        let checks = serde_json::json!([
            {"name": "daemon_version", "passed": false, "detail": "mismatch: Cargo=1.0.0 VERSION.md=0.9.0"},
            {"name": "telegram", "passed": true, "detail": "ok"}
        ]);
        let critical = classify_readiness_results(&pool, "node-gamma", &checks);
        assert!(critical, "daemon_version failure must produce CRITICAL event");
    }

    #[test]
    fn classify_readiness_no_events_when_all_pass() {
        use r2d2::Pool;
        use r2d2_sqlite::SqliteConnectionManager;

        let mgr = SqliteConnectionManager::memory();
        let pool = Pool::new(mgr).expect("pool");
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
        use r2d2::Pool;
        use r2d2_sqlite::SqliteConnectionManager;
        use crate::kernel::monitor::check_peer_readiness;

        let mgr = SqliteConnectionManager::memory();
        let pool = Pool::new(mgr).expect("pool");
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

    // --- deprecated watchdog compat ---

    #[test]
    #[allow(deprecated)]
    fn deprecated_watchdog_decide_action_compiles() {
        use crate::resilience::watchdog::{decide_action, CheckResult, WatchdogAction};
        let failures = vec![CheckResult::fail("daemon_health", "down")];
        assert_eq!(decide_action(&failures), WatchdogAction::Restart);
    }

    #[test]
    #[allow(deprecated)]
    fn deprecated_watchdog_config_struct_compiles() {
        use crate::resilience::watchdog::WatchdogConfig;
        let cfg = WatchdogConfig::default();
        assert_eq!(cfg.check_interval_secs, 30);
        assert!(!cfg.daemon_url.is_empty());
    }
}
