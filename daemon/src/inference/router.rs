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
mod tests;
