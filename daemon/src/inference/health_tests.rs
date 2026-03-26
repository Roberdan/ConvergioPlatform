#[cfg(test)]
mod tests {
    use crate::inference::health::{EndpointHealth, EndpointHealthStatus, HealthChecker, ProbeResult};
    use std::time::Duration;

    // Helper: build an EndpointHealth with N recorded latencies and M error probes.
    fn make_endpoint(name: &str) -> EndpointHealth {
        EndpointHealth::new(name)
    }

    // ── Healthy endpoint ─────────────────────────────────────────────────────

    #[test]
    fn test_status_healthy_when_no_data() {
        let ep = make_endpoint("local-omlx");
        assert_eq!(ep.status(), EndpointHealthStatus::Healthy);
    }

    #[test]
    fn test_status_healthy_with_low_latency() {
        let mut ep = make_endpoint("litellm");
        for _ in 0..5 {
            ep.record_probe(ProbeResult::Success(Duration::from_millis(500)));
        }
        assert_eq!(ep.status(), EndpointHealthStatus::Healthy);
    }

    // ── Degraded by latency ──────────────────────────────────────────────────

    #[test]
    fn test_status_degraded_when_avg_latency_exceeds_2s() {
        let mut ep = make_endpoint("cloud-api");
        // Record 5 probes with 3000ms latency → avg > 2000ms
        for _ in 0..5 {
            ep.record_probe(ProbeResult::Success(Duration::from_millis(3000)));
        }
        let status = ep.status();
        assert!(
            matches!(status, EndpointHealthStatus::Degraded(_)),
            "expected Degraded, got {:?}",
            status
        );
    }

    #[test]
    fn test_status_degraded_reason_contains_latency() {
        let mut ep = make_endpoint("cloud-api");
        for _ in 0..5 {
            ep.record_probe(ProbeResult::Success(Duration::from_millis(2500)));
        }
        if let EndpointHealthStatus::Degraded(reason) = ep.status() {
            assert!(
                reason.contains("latency"),
                "reason should mention latency: {reason}"
            );
        } else {
            panic!("expected Degraded");
        }
    }

    // ── Degraded by error rate ───────────────────────────────────────────────

    #[test]
    fn test_status_degraded_when_error_rate_exceeds_5pct() {
        let mut ep = make_endpoint("cloud-api");
        // 1 error out of 10 probes = 10% error rate → Degraded
        for _ in 0..9 {
            ep.record_probe(ProbeResult::Success(Duration::from_millis(200)));
        }
        ep.record_probe(ProbeResult::Error("timeout".into()));
        let status = ep.status();
        assert!(
            matches!(status, EndpointHealthStatus::Degraded(_)),
            "expected Degraded for 10% error rate, got {:?}",
            status
        );
    }

    // ── Down after consecutive failures ─────────────────────────────────────

    #[test]
    fn test_status_down_after_3_consecutive_failures() {
        let mut ep = make_endpoint("local-omlx");
        for _ in 0..3 {
            ep.record_probe(ProbeResult::Error("connection refused".into()));
        }
        assert_eq!(ep.status(), EndpointHealthStatus::Down);
    }

    #[test]
    fn test_consecutive_failures_reset_on_success() {
        let mut ep = make_endpoint("local-omlx");
        for _ in 0..2 {
            ep.record_probe(ProbeResult::Error("timeout".into()));
        }
        // Recovery: one success resets consecutive counter
        ep.record_probe(ProbeResult::Success(Duration::from_millis(100)));
        // Only 0 consecutive failures now — not Down
        assert_ne!(ep.status(), EndpointHealthStatus::Down);
    }

    // ── HealthChecker aggregation ────────────────────────────────────────────

    #[test]
    fn test_health_checker_status_unknown_endpoint_returns_healthy() {
        let checker = HealthChecker::new(vec![]);
        // Unknown endpoint defaults to Healthy (no data = no problems known)
        assert_eq!(
            checker.status("nonexistent"),
            EndpointHealthStatus::Healthy
        );
    }

    #[test]
    fn test_health_checker_status_reflects_endpoint_state() {
        let mut ep = make_endpoint("litellm");
        for _ in 0..3 {
            ep.record_probe(ProbeResult::Error("refused".into()));
        }
        let checker = HealthChecker::from_endpoints(vec![ep]);
        assert_eq!(checker.status("litellm"), EndpointHealthStatus::Down);
    }

    // ── Ring buffer bounded ──────────────────────────────────────────────────

    #[test]
    fn test_latency_ring_buffer_bounded_to_20_entries() {
        let mut ep = make_endpoint("local-omlx");
        // Push 30 successful probes at 100ms
        for _ in 0..30 {
            ep.record_probe(ProbeResult::Success(Duration::from_millis(100)));
        }
        // Still healthy — ring buffer should not panic
        assert_eq!(ep.status(), EndpointHealthStatus::Healthy);
    }
}
