// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// TDD tests for kernel/monitor.rs — written BEFORE implementation (RED phase).
// Verifies that deprecated watchdog functions still compile and that the kernel
// monitor provides equivalent health-check and stale-task detection.

#[cfg(test)]
mod tests {
    use crate::kernel::monitor::{check_daemon_reachable, detect_stale_locks, KernelCheckResult};

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

    #[test]
    fn detect_stale_locks_returns_check_result() {
        // Stale-lock detection must return a KernelCheckResult (not panic).
        let result = detect_stale_locks(300);
        // Result may pass or fail depending on environment; structural check only.
        assert!(!result.check_name.is_empty());
    }

    #[tokio::test]
    async fn check_daemon_reachable_unreachable_host_fails_gracefully() {
        // Port 19999 is unlikely to be in use; must fail with details, not panic.
        let result = check_daemon_reachable("http://127.0.0.1:19999").await;
        assert!(!result.ok);
        assert!(result.details.is_some());
    }

    // Verify deprecated watchdog symbols still compile (lint silenced for test scope).
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
        // Ensure the deprecated WatchdogConfig type is still importable and usable.
        use crate::resilience::watchdog::WatchdogConfig;
        let cfg = WatchdogConfig::default();
        assert_eq!(cfg.check_interval_secs, 30);
        assert!(!cfg.daemon_url.is_empty());
    }
}
