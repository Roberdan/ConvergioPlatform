// Background health probe loop for inference providers.
// Probes configured endpoints every 60s and stores results in shared state.

use super::health::HealthChecker;
use std::sync::Arc;
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

/// Spawn background task that probes provider health every 60s.
/// Waits 10s before the first probe to allow services to start.
pub fn spawn_health_probe_loop(state: SharedHealthState) {
    tokio::spawn(async move {
        // Give services time to start before first probe.
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        loop {
            let local_port =
                std::env::var("LOCAL_LLM_PORT").unwrap_or_else(|_| "8321".into());

            // Probe local LLM via HTTP.
            let local_url = format!("http://localhost:{local_port}/v1/models");
            {
                let mut checker = state.write().await;
                checker.probe_http("local-llm", &local_url).await;
            }

            // Claude: CLI-based; verify the local binary exists — no external HTTP.
            // Hitting the Anthropic API is wrong: we use subscription (no API key),
            // and an external call every 60s adds unnecessary load.
            {
                let mut checker = state.write().await;
                checker.probe_cli("claude", "claude").await;
            }

            // Copilot: CLI-based; verify the binary exists locally.
            {
                let mut checker = state.write().await;
                checker.probe_cli("copilot", "copilot").await;
            }

            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
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
