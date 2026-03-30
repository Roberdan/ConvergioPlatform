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

            // Claude: CLI-based; probe the Anthropic API as reachability signal.
            let claude_cli_ok = tokio::process::Command::new("which")
                .arg("claude")
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false);

            if claude_cli_ok {
                let mut checker = state.write().await;
                checker
                    .probe_http("claude", "https://api.anthropic.com/v1/models")
                    .await;
            }

            // Copilot: CLI-based (gh); no public health endpoint, skip HTTP probe.
            // Status is inferred at read-time from CLI availability in the handler.

            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    });
}
