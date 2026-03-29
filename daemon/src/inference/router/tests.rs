use super::*;
use crate::inference::types::{InferenceConstraints, InferenceTier, ModelProvider};

fn make_endpoint(
    name: &str,
    provider: ModelProvider,
    low: InferenceTier,
    high: InferenceTier,
    healthy: bool,
) -> ModelEndpoint {
    ModelEndpoint {
        name: name.to_string(),
        provider,
        url: format!("http://localhost/{}", name),
        tier_range: (low, high),
        healthy,
    }
}

fn constraints_none() -> InferenceConstraints {
    InferenceConstraints {
        max_latency_ms: None,
        max_cost: None,
    }
}

fn request(tier: InferenceTier) -> InferenceRequest {
    InferenceRequest {
        prompt: "Hello world".to_string(),
        max_tokens: 256,
        tier_hint: Some(tier),
        agent_id: "test-agent".to_string(),
        constraints: constraints_none(),
    }
}

// --- F-01: route to correct tier ---

#[test]
fn routes_t1_trivial_to_registered_model() {
    let mut router = InferenceRouter::new();
    router.register_model(make_endpoint(
        "gemma-3-1b",
        ModelProvider::Local,
        InferenceTier::T1Trivial,
        InferenceTier::T2Standard,
        true,
    ));
    let resp = router.route(&request(InferenceTier::T1Trivial)).unwrap();
    assert_eq!(resp.model_used, "gemma-3-1b");
}

#[test]
fn routes_t3_complex_to_matching_model() {
    let mut router = InferenceRouter::new();
    router.register_model(make_endpoint(
        "llama3-70b",
        ModelProvider::Local,
        InferenceTier::T3Complex,
        InferenceTier::T4Critical,
        true,
    ));
    let resp = router.route(&request(InferenceTier::T3Complex)).unwrap();
    assert_eq!(resp.model_used, "llama3-70b");
}

#[test]
fn defaults_to_t2_when_no_tier_hint() {
    let mut router = InferenceRouter::new();
    router.register_model(make_endpoint(
        "mistral-7b",
        ModelProvider::Local,
        InferenceTier::T1Trivial,
        InferenceTier::T4Critical,
        true,
    ));
    let req = InferenceRequest {
        prompt: "test".to_string(),
        max_tokens: 64,
        tier_hint: None,
        agent_id: "agent-x".to_string(),
        constraints: constraints_none(),
    };
    let resp = router.route(&req).unwrap();
    assert_eq!(resp.model_used, "mistral-7b");
}

// --- F-01: health filtering ---

#[test]
fn skips_unhealthy_model() {
    let mut router = InferenceRouter::new();
    router.register_model(make_endpoint(
        "broken-model",
        ModelProvider::Local,
        InferenceTier::T1Trivial,
        InferenceTier::T4Critical,
        false,
    ));
    router.register_model(make_endpoint(
        "healthy-model",
        ModelProvider::Cloud,
        InferenceTier::T1Trivial,
        InferenceTier::T4Critical,
        true,
    ));
    let resp = router.route(&request(InferenceTier::T2Standard)).unwrap();
    assert_eq!(resp.model_used, "healthy-model");
}

#[test]
fn returns_error_when_no_healthy_model() {
    let mut router = InferenceRouter::new();
    router.register_model(make_endpoint(
        "offline",
        ModelProvider::Local,
        InferenceTier::T1Trivial,
        InferenceTier::T4Critical,
        false,
    ));
    let result = router.route(&request(InferenceTier::T2Standard));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no healthy model"));
}

// --- F-01: health_update ---

#[test]
fn health_update_makes_model_available() {
    let mut router = InferenceRouter::new();
    router.register_model(make_endpoint(
        "recovering",
        ModelProvider::Local,
        InferenceTier::T1Trivial,
        InferenceTier::T4Critical,
        false,
    ));
    assert!(router.route(&request(InferenceTier::T2Standard)).is_err());
    router.health_update("recovering", true);
    let resp = router.route(&request(InferenceTier::T2Standard)).unwrap();
    assert_eq!(resp.model_used, "recovering");
}

// --- F-01: fallback chain ---

#[test]
fn fallback_chain_contains_secondary_models() {
    let mut router = InferenceRouter::new();
    router.register_model(make_endpoint(
        "primary-local",
        ModelProvider::Local,
        InferenceTier::T1Trivial,
        InferenceTier::T4Critical,
        true,
    ));
    router.register_model(make_endpoint(
        "secondary-cloud",
        ModelProvider::Cloud,
        InferenceTier::T1Trivial,
        InferenceTier::T4Critical,
        true,
    ));
    let decision = router
        .select(&InferenceTier::T2Standard, &constraints_none())
        .unwrap();
    assert_eq!(decision.selected_model, "primary-local");
    assert!(decision.fallback_chain.contains(&"secondary-cloud".to_string()));
}

// --- F-01: tier mismatch returns error ---

#[test]
fn returns_error_when_tier_not_covered() {
    let mut router = InferenceRouter::new();
    router.register_model(make_endpoint(
        "tiny-model",
        ModelProvider::Local,
        InferenceTier::T1Trivial,
        InferenceTier::T1Trivial,
        true,
    ));
    let result = router.route(&request(InferenceTier::T4Critical));
    assert!(result.is_err());
}
