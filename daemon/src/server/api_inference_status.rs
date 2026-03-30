// GET /api/inference/status — active providers, health, and configuration.

use super::state::{ApiError, ServerState};
use axum::extract::State;
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
    description: String,
}

#[derive(Serialize)]
struct InferenceStatus {
    providers: Vec<ProviderStatus>,
    default_model: String,
    fallback_chains: Value,
}

fn check_cli_available(cmd: &str) -> bool {
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
) -> Result<Json<InferenceStatus>, ApiError> {
    let claude_ok = check_cli_available("claude");
    let gh_ok = check_cli_available("gh");

    let local_port = std::env::var("LOCAL_LLM_PORT").unwrap_or_else(|_| "8321".into());
    let local_ok = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default()
        .get(format!("http://localhost:{local_port}/v1/models"))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    let providers = vec![
        ProviderStatus {
            name: "ClaudeSubscription".into(),
            provider_type: "cli_subprocess".into(),
            available: claude_ok,
            description: "claude -p (OAuth subscription)".into(),
        },
        ProviderStatus {
            name: "CopilotSubscription".into(),
            provider_type: "cli_subprocess".into(),
            available: gh_ok,
            description: "gh copilot -p (GitHub subscription)".into(),
        },
        ProviderStatus {
            name: "LocalLLM".into(),
            provider_type: "http_openai_compat".into(),
            available: local_ok,
            description: format!("localhost:{local_port} (Ollama/MLX)"),
        },
    ];

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
