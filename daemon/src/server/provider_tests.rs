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
