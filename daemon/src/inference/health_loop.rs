// Background health probe loop for inference providers.
// Probes configured endpoints every 60s and stores results in shared state.

use super::health::{HealthChecker, ProbeResult};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Shared health state accessible from API handlers.
pub type SharedHealthState = Arc<RwLock<HealthChecker>>;

/// Create the shared health state with default provider endpoints.
pub fn create_shared_health() -> SharedHealthState {
    Arc::new(RwLock::new(HealthChecker::new(vec![
        "claude",
        "copilot",
        "local-llm",
    ])))
}

/// Run a CLI probe without holding any lock.
/// WHY: holding a write lock across async I/O causes contention and deadlock
///      risk (GPT-5.4 audit). Collect results first, then write briefly.
async fn probe_cli_no_lock(cmd: &str) -> ProbeResult {
    let start = std::time::Instant::now();
    match tokio::process::Command::new("which").arg(cmd).output().await {
        Ok(o) if o.status.success() => ProbeResult::Success(start.elapsed()),
        Ok(o) => ProbeResult::Error(format!(
            "which {cmd} exited {}",
            o.status.code().unwrap_or(-1)
        )),
        Err(e) => ProbeResult::Error(e.to_string()),
    }
}

/// Run an HTTP probe without holding any lock.
async fn probe_http_no_lock(url: &str) -> ProbeResult {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_default();
    let start = std::time::Instant::now();
    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => ProbeResult::Success(start.elapsed()),
        Ok(resp) => ProbeResult::Error(format!("HTTP {}", resp.status())),
        Err(e) => ProbeResult::Error(e.to_string()),
    }
}

/// Spawn background task that probes provider health every 60s.
/// Waits 10s before the first probe to allow services to start.
pub fn spawn_health_probe_loop(state: SharedHealthState) {
    tokio::spawn(async move {
        // Give services time to start before first probe.
        tokio::time::sleep(Duration::from_secs(10)).await;

        loop {
            let local_port =
                std::env::var("LOCAL_LLM_PORT").unwrap_or_else(|_| "8321".into());
            let local_url = format!("http://localhost:{local_port}/v1/models");

            // Run all probes OUTSIDE the lock — I/O must never hold a write lock.
            let local_result = probe_http_no_lock(&local_url).await;
            let claude_result = probe_cli_no_lock("claude").await;
            let copilot_result = probe_cli_no_lock("copilot").await;

            // Acquire write lock briefly only to record collected results.
            {
                let mut checker = state.write().await;
                checker.record_result("local-llm", local_result);
                checker.record_result("claude", claude_result);
                checker.record_result("copilot", copilot_result);
            }

            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_shared_health_has_three_endpoints() {
        let state = create_shared_health();
        // Use try_read to access without async runtime — state just created, no writers.
        let checker = state.try_read().expect("lock must be uncontested");
        let names = checker.endpoint_names();
        assert_eq!(names.len(), 3, "expected 3 endpoints, got: {names:?}");
        assert!(names.contains(&"claude".to_string()), "missing claude");
        assert!(names.contains(&"copilot".to_string()), "missing copilot");
        assert!(names.contains(&"local-llm".to_string()), "missing local-llm");
    }

    #[test]
    fn test_shared_health_state_can_be_cloned() {
        let state = create_shared_health();
        let clone = Arc::clone(&state);
        // Both handles must point to the same underlying RwLock.
        let names_orig = state.try_read().unwrap().endpoint_names();
        let names_clone = clone.try_read().unwrap().endpoint_names();
        assert_eq!(names_orig, names_clone);
    }

    #[test]
    fn test_shared_health_state_initial_endpoints_healthy() {
        let state = create_shared_health();
        let checker = state.try_read().unwrap();
        for name in &["claude", "copilot", "local-llm"] {
            let status = checker.status(name);
            // No probes recorded yet — should be Healthy.
            assert_eq!(
                status,
                crate::inference::health::EndpointHealthStatus::Healthy,
                "endpoint '{name}' must be Healthy before any probes"
            );
        }
    }
}
