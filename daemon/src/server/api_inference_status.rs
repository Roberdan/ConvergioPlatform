// GET /api/inference/status — active providers, health, and configuration.
// Reads health data from the background probe loop via SharedHealthState.

use crate::inference::health::EndpointHealthStatus;
use crate::inference::health_loop::SharedHealthState;
use super::state::{ApiError, ServerState};
use axum::extract::{Extension, State};
use axum::response::Json;
use axum::Router;
use axum::routing::get;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
struct ProviderStatus {
    name: String,
    provider_type: String,
    available: bool,
    status: String,
    latency_ms: u64,
    description: String,
}

#[derive(Serialize)]
struct InferenceStatus {
    providers: Vec<ProviderStatus>,
    default_model: String,
    fallback_chains: Value,
}

fn health_label(s: &EndpointHealthStatus) -> (&'static str, bool) {
    match s {
        EndpointHealthStatus::Healthy => ("healthy", true),
        EndpointHealthStatus::Degraded(_) => ("degraded", true),
        EndpointHealthStatus::Down => ("down", false),
    }
}

fn cli_available(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn inference_status_handler(
    State(_state): State<ServerState>,
    Extension(health): Extension<SharedHealthState>,
) -> Result<Json<InferenceStatus>, ApiError> {
    let checker = health.read().await;

    let local_port = std::env::var("LOCAL_LLM_PORT").unwrap_or_else(|_| "8321".into());

    // Claude: health from background probe + CLI availability guard.
    let claude_status = checker.status("claude");
    let (claude_label, claude_avail) = health_label(&claude_status);
    let claude_ok = claude_avail && cli_available("claude");

    // Copilot: no HTTP probe; derive from CLI availability only.
    let gh_ok = cli_available("gh");
    let copilot_label = if gh_ok { "healthy" } else { "down" };

    // Local LLM: health entirely from background probe.
    let local_status = checker.status("local-llm");
    let (local_label, local_avail) = health_label(&local_status);

    let providers = vec![
        ProviderStatus {
            name: "ClaudeSubscription".into(),
            provider_type: "cli_subprocess".into(),
            available: claude_ok,
            status: claude_label.into(),
            latency_ms: checker.latency_ms("claude"),
            description: "claude -p (OAuth subscription)".into(),
        },
        ProviderStatus {
            name: "CopilotSubscription".into(),
            provider_type: "cli_subprocess".into(),
            available: gh_ok,
            status: copilot_label.into(),
            latency_ms: 0,
            description: "gh copilot -p (GitHub subscription)".into(),
        },
        ProviderStatus {
            name: "LocalLLM".into(),
            provider_type: "http_openai_compat".into(),
            available: local_avail,
            status: local_label.into(),
            latency_ms: checker.latency_ms("local-llm"),
            description: format!("localhost:{local_port} (Ollama/MLX)"),
        },
    ];

    drop(checker);

    let chains = serde_json::json!({
        "t1": ["local", "haiku", "sonnet"],
        "t2": ["haiku", "local", "sonnet"],
        "t3": ["sonnet", "opus"],
        "t4": ["opus", "sonnet"],
    });

    Ok(Json(InferenceStatus {
        providers,
        default_model: "claude-sonnet-4-20250514".into(),
        fallback_chains: chains,
    }))
}

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/inference/status", get(inference_status_handler))
}
