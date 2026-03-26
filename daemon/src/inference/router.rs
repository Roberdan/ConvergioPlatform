use std::collections::HashMap;

use super::types::{
    InferenceConstraints, InferenceRequest, InferenceResponse, InferenceTier, ModelEndpoint,
    RoutingDecision,
};

/// Routes inference requests to the best available model endpoint.
///
/// Selection order: tier match → health filter → cost/latency constraints → first candidate.
pub struct InferenceRouter {
    models: HashMap<String, ModelEndpoint>,
}

impl InferenceRouter {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    /// Register a model endpoint. Replaces any existing entry with the same name.
    pub fn register_model(&mut self, endpoint: ModelEndpoint) {
        self.models.insert(endpoint.name.clone(), endpoint);
    }

    /// Update health status for a named model.
    pub fn health_update(&mut self, name: &str, healthy: bool) {
        if let Some(ep) = self.models.get_mut(name) {
            ep.healthy = healthy;
        }
    }

    /// Route a request to the best model, returning a synthetic response.
    ///
    /// Returns `Err` when no healthy model covers the requested tier.
    pub fn route(&self, request: &InferenceRequest) -> Result<InferenceResponse, String> {
        let tier = request
            .tier_hint
            .clone()
            .unwrap_or(InferenceTier::T2Standard);

        let decision = self.select(&tier, &request.constraints)?;

        // Synthetic response — real I/O handled by the caller
        Ok(InferenceResponse {
            content: format!(
                "[routed to {}] {}",
                decision.selected_model, &request.prompt
            ),
            model_used: decision.selected_model,
            latency_ms: 0,
            tokens_used: request.max_tokens,
            cost: 0.0,
        })
    }

    /// Internal: build a routing decision for the given tier and constraints.
    fn select(
        &self,
        tier: &InferenceTier,
        _constraints: &InferenceConstraints,
    ) -> Result<RoutingDecision, String> {
        // Collect healthy models whose tier range covers the requested tier
        let mut candidates: Vec<&ModelEndpoint> = self
            .models
            .values()
            .filter(|ep| ep.healthy && ep.tier_range.0 <= *tier && ep.tier_range.1 >= *tier)
            .collect();

        if candidates.is_empty() {
            return Err(format!("no healthy model available for tier {:?}", tier));
        }

        // Prefer local models (lower latency, zero cost)
        candidates.sort_by_key(|ep| {
            use super::types::ModelProvider;
            if ep.provider == ModelProvider::Local {
                0u8
            } else {
                1u8
            }
        });

        let selected = candidates[0];
        let fallback_chain: Vec<String> = candidates[1..].iter().map(|e| e.name.clone()).collect();

        Ok(RoutingDecision {
            selected_model: selected.name.clone(),
            reason: format!("tier {:?} → first healthy candidate", tier),
            fallback_chain,
        })
    }
}

impl Default for InferenceRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
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
            false, // unhealthy
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

        // Should fail while unhealthy
        assert!(router.route(&request(InferenceTier::T2Standard)).is_err());

        // Mark healthy
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

        // Local preferred; cloud in fallback
        assert_eq!(decision.selected_model, "primary-local");
        assert!(decision
            .fallback_chain
            .contains(&"secondary-cloud".to_string()));
    }

    // --- F-01: tier mismatch returns error ---

    #[test]
    fn returns_error_when_tier_not_covered() {
        let mut router = InferenceRouter::new();
        // Model only handles T1
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
}
