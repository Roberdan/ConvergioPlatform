use serde::{Deserialize, Serialize};

/// Model tier classification — maps to capability requirements
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum InferenceTier {
    T1Trivial,
    T2Standard,
    T3Complex,
    T4Critical,
}

/// Routing constraints from the caller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConstraints {
    pub max_latency_ms: Option<u64>,
    pub max_cost: Option<f64>,
}

/// Incoming inference request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub prompt: String,
    pub max_tokens: u32,
    /// Caller hint — router may override based on health/budget
    pub tier_hint: Option<InferenceTier>,
    pub agent_id: String,
    pub constraints: InferenceConstraints,
}

/// Result returned after routing and (mock) inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub content: String,
    pub model_used: String,
    pub latency_ms: u64,
    pub tokens_used: u32,
    pub cost: f64,
}

/// Provider classification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelProvider {
    Local,
    Cloud,
}

/// A registered model endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEndpoint {
    pub name: String,
    pub provider: ModelProvider,
    pub url: String,
    /// Inclusive tier range this model serves
    pub tier_range: (InferenceTier, InferenceTier),
    pub healthy: bool,
}

/// The router's internal routing decision (used for logging / fallback chains)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub selected_model: String,
    pub reason: String,
    pub fallback_chain: Vec<String>,
}
