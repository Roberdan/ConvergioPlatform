// Provider routing and fallback logic for LLM inference.
// Maps model names to providers (CLI subscriptions / local LLM) and
// executes fallback chains via inference/fallback.rs tiers.

use super::llm_client::{self, ChatMessage, Provider, StreamChunk};
use crate::inference::fallback::FallbackConfig;
use crate::inference::types::InferenceTier;
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::warn;

/// Resolve a model name to a concrete provider + model string.
pub fn provider_for_model(model: &str) -> (Provider, String) {
    let lower = model.to_lowercase();

    if lower.starts_with("local/")
        || lower.contains("ollama")
        || lower.contains("mlx")
        || lower.contains("llama")
        || lower.contains("gemma")
        || lower.contains("mistral")
        || lower.contains("qwen")
    {
        return (Provider::LocalLLM, model.to_string());
    }

    if lower.contains("claude") || lower.contains("opus")
        || lower.contains("sonnet") || lower.contains("haiku")
    {
        return (Provider::ClaudeSubscription, model.to_string());
    }

    if lower.contains("gpt") || lower.contains("copilot") || lower.contains("o1") {
        return (Provider::CopilotSubscription, model.to_string());
    }

    (Provider::ClaudeSubscription, model.to_string())
}

/// Map a model name to an inference tier for fallback chain selection.
pub fn tier_for_model(model: &str) -> InferenceTier {
    let lower = model.to_lowercase();
    if lower.contains("opus") || lower.contains("o1") {
        InferenceTier::T4Critical
    } else if lower.contains("sonnet") || lower.contains("gpt-4") {
        InferenceTier::T3Complex
    } else if lower.contains("haiku") || lower.contains("gpt-3") {
        InferenceTier::T2Standard
    } else if lower.contains("local") || lower.contains("llama")
        || lower.contains("gemma") || lower.contains("mlx")
    {
        InferenceTier::T1Trivial
    } else {
        InferenceTier::T2Standard
    }
}

/// Resolve fallback chain model name to a concrete provider.
pub fn provider_for_fallback(name: &str) -> (Provider, String) {
    match name {
        "local" => (Provider::LocalLLM, "default".to_string()),
        "haiku" => (Provider::ClaudeSubscription, "claude-haiku-4-20250506".into()),
        "sonnet" => (Provider::ClaudeSubscription, "claude-sonnet-4-20250514".into()),
        "opus" => (Provider::ClaudeSubscription, "claude-opus-4-20250514".into()),
        other => provider_for_model(other),
    }
}

/// Cost estimate per 1k tokens (rough defaults for budget visibility).
pub fn estimate_cost(model: &str, input_tokens: u64, output_tokens: u64) -> f64 {
    let (in_rate, out_rate) = if model.contains("opus") {
        (0.015, 0.075)
    } else if model.contains("sonnet") {
        (0.003, 0.015)
    } else if model.contains("haiku") {
        (0.00025, 0.00125)
    } else if model.contains("local") || model.contains("llama") {
        (0.0, 0.0)
    } else {
        (0.002, 0.01)
    };
    (input_tokens as f64 * in_rate + output_tokens as f64 * out_rate) / 1000.0
}

/// Try primary provider; on failure, walk the fallback chain for the tier.
pub fn stream_with_fallback(
    primary: Provider,
    model: &str,
    messages: Vec<ChatMessage>,
) -> llm_client::ChatStream {
    let tier = tier_for_model(model);
    let config = FallbackConfig::default_chains();
    let chain = config.chain_for(&tier);
    let primary_model = model.to_string();
    let msgs = messages.clone();

    if chain.is_empty() {
        return llm_client::stream_chat(primary, model, messages);
    }

    let chain_owned: Vec<String> = chain.to_vec();
    let max_attempts = config.max_attempts();
    let (tx, rx) = mpsc::channel::<StreamChunk>(64);

    tokio::spawn(async move {
        let primary_stream = llm_client::stream_chat(primary, &primary_model, msgs.clone());
        let result = collect_stream_result(primary_stream).await;

        match result {
            Ok(chunks) => {
                for chunk in chunks {
                    if tx.send(chunk).await.is_err() { return; }
                }
            }
            Err(err) => {
                warn!(model = %primary_model, error = %err, "primary LLM failed, trying fallback");
                let mut succeeded = false;
                for (idx, fb_name) in chain_owned.iter().enumerate() {
                    if idx >= max_attempts { break; }
                    let (fb_provider, fb_model) = provider_for_fallback(fb_name);
                    let fb_stream = llm_client::stream_chat(fb_provider, &fb_model, msgs.clone());
                    match collect_stream_result(fb_stream).await {
                        Ok(chunks) => {
                            for chunk in chunks {
                                if tx.send(chunk).await.is_err() { return; }
                            }
                            succeeded = true;
                            break;
                        }
                        Err(fb_err) => {
                            warn!(model = %fb_model, attempt = idx + 1, error = %fb_err, "fallback failed");
                        }
                    }
                }
                if !succeeded {
                    // intentional: best-effort error delivery to client
                    let _ = tx.send(StreamChunk::Error(
                        format!("all providers failed. Last: {err}")
                    )).await;
                }
            }
        }
    });

    Box::pin(ReceiverStream::new(rx))
}

async fn collect_stream_result(
    mut stream: llm_client::ChatStream,
) -> Result<Vec<StreamChunk>, String> {
    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        if let StreamChunk::Error(ref e) = chunk {
            return Err(e.clone());
        }
        chunks.push(chunk);
    }
    if chunks.is_empty() {
        return Err("empty response from LLM".into());
    }
    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_claude_models_to_subscription() {
        let (p, _) = provider_for_model("claude-sonnet-4-20250514");
        assert_eq!(p, Provider::ClaudeSubscription);
    }

    #[test]
    fn routes_opus_to_subscription() {
        let (p, _) = provider_for_model("claude-opus-4-20250514");
        assert_eq!(p, Provider::ClaudeSubscription);
    }

    #[test]
    fn routes_gpt_to_copilot() {
        let (p, _) = provider_for_model("gpt-4o");
        assert_eq!(p, Provider::CopilotSubscription);
    }

    #[test]
    fn routes_local_models() {
        let (p, _) = provider_for_model("local/llama3");
        assert_eq!(p, Provider::LocalLLM);
        let (p2, _) = provider_for_model("ollama-mistral");
        assert_eq!(p2, Provider::LocalLLM);
    }

    #[test]
    fn tier_classification() {
        assert_eq!(tier_for_model("claude-opus-4"), InferenceTier::T4Critical);
        assert_eq!(tier_for_model("claude-sonnet-4"), InferenceTier::T3Complex);
        assert_eq!(tier_for_model("claude-haiku-4"), InferenceTier::T2Standard);
        assert_eq!(tier_for_model("local/llama3"), InferenceTier::T1Trivial);
    }

    #[test]
    fn fallback_resolves_known_names() {
        let (p, m) = provider_for_fallback("sonnet");
        assert_eq!(p, Provider::ClaudeSubscription);
        assert!(m.contains("sonnet"));

        let (p2, _) = provider_for_fallback("local");
        assert_eq!(p2, Provider::LocalLLM);
    }

    #[test]
    fn cost_estimate_local_is_zero() {
        assert_eq!(estimate_cost("local/llama3", 1000, 1000), 0.0);
    }

    #[test]
    fn cost_estimate_opus_higher_than_haiku() {
        let opus = estimate_cost("opus", 1000, 1000);
        let haiku = estimate_cost("haiku", 1000, 1000);
        assert!(opus > haiku);
    }
}
