// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for kernel/monitor_checks.rs

#[cfg(test)]
mod tests {
    use crate::kernel::monitor::KernelCheckResult;
    use crate::kernel::monitor_checks::detect_stale_locks;

    #[test]
    fn detect_stale_locks_passes_when_no_stale() {
        // Using very large threshold so nothing is stale
        let result = detect_stale_locks(999_999);
        assert!(result.ok, "expected pass: {:?}", result.details);
    }

    #[test]
    fn detect_stale_locks_returns_kernel_check_result() {
        let result = detect_stale_locks(300);
        assert_eq!(result.check_name, "stale_locks");
    }

    // check_api_telemetry requires a running daemon — tested via integration tests.
    // Verify the function signature compiles and is accessible.
    #[tokio::test]
    async fn check_api_telemetry_fails_on_unreachable() {
        let result =
            crate::kernel::monitor_checks::check_api_telemetry("http://127.0.0.1:19999").await;
        assert!(!result.ok, "should fail when daemon unreachable");
        assert_eq!(result.check_name, "api_telemetry");
    }
}
