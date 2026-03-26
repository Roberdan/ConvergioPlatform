use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::inference::types::{InferenceRequest, InferenceTier};

// ── Tier YAML deserialization ─────────────────────────────────────────────────

/// Deserializes short tier strings (t1..t4) from YAML config files.
/// InferenceTier's derived Deserialize handles enum variant names; this
/// custom deserializer maps the compact lowercase aliases used in agent configs.
fn deserialize_tier<'de, D>(de: D) -> Result<InferenceTier, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(de)?;
    match s.to_ascii_lowercase().as_str() {
        "t1" | "t1trivial" => Ok(InferenceTier::T1Trivial),
        "t2" | "t2standard" => Ok(InferenceTier::T2Standard),
        "t3" | "t3complex" => Ok(InferenceTier::T3Complex),
        "t4" | "t4critical" => Ok(InferenceTier::T4Critical),
        other => Err(serde::de::Error::unknown_variant(
            other,
            &["t1", "t2", "t3", "t4"],
        )),
    }
}

// ── AgentInferenceConfig ──────────────────────────────────────────────────────

/// Per-agent inference constraints loaded from `config/agents/{name}/inference.yaml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentInferenceConfig {
    /// Preferred model name, if the agent has a preference.
    pub preferred_model: Option<String>,

    /// Hard ceiling on the tier this agent may use.
    #[serde(deserialize_with = "deserialize_tier")]
    pub max_tier: InferenceTier,

    /// Daily token budget; exceeding it triggers a one-tier downgrade.
    pub budget_tokens_per_day: u64,

    /// Latency SLA for routing decisions (ms).
    pub latency_sla_ms: u64,

    /// Allowlist of model names this agent may use; empty = unrestricted.
    pub allowed_models: Vec<String>,
}

// ── AgentConfigRegistry ───────────────────────────────────────────────────────

/// In-memory registry of per-agent configs keyed by agent identifier.
pub struct AgentConfigRegistry {
    configs: HashMap<String, AgentInferenceConfig>,
}

impl AgentConfigRegistry {
    /// Scan `path` for `<agent-name>/inference.yaml` files and load them all.
    /// Files that fail to parse are logged and skipped (fail-soft).
    pub fn load_directory(path: &Path) -> Result<Self, String> {
        let mut configs = HashMap::new();

        let entries = std::fs::read_dir(path)
            .map_err(|e| format!("cannot read agent config dir {}: {e}", path.display()))?;

        for entry in entries.flatten() {
            let agent_dir = entry.path();
            if !agent_dir.is_dir() {
                continue;
            }
            let yaml_path = agent_dir.join("inference.yaml");
            if !yaml_path.exists() {
                continue;
            }
            let agent_name = match agent_dir.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            let content = std::fs::read_to_string(&yaml_path)
                .map_err(|e| format!("cannot read {}: {e}", yaml_path.display()))?;
            match serde_yaml::from_str::<AgentInferenceConfig>(&content) {
                Ok(cfg) => {
                    configs.insert(agent_name, cfg);
                }
                Err(e) => {
                    warn!("skipping malformed config {}: {e}", yaml_path.display());
                }
            }
        }

        Ok(Self { configs })
    }

    /// Look up the config for a specific agent.
    pub fn get(&self, agent_id: &str) -> Option<&AgentInferenceConfig> {
        self.configs.get(agent_id)
    }

    /// Permissive defaults used when an agent has no explicit config file.
    pub fn default_config() -> AgentInferenceConfig {
        AgentInferenceConfig {
            preferred_model: None,
            max_tier: InferenceTier::T4Critical,
            budget_tokens_per_day: 10_000_000,
            latency_sla_ms: 30_000,
            allowed_models: vec![],
        }
    }
}

// ── BudgetTracker ─────────────────────────────────────────────────────────────

/// In-memory daily token usage counters per agent.
///
/// Intentionally simple: counters reset when the process restarts. A
/// persistent implementation would back this with the DB, but that is
/// out of scope for T2-03 (tracked separately).
pub struct BudgetTracker {
    usage: HashMap<String, u64>,
}

impl BudgetTracker {
    pub fn new() -> Self {
        Self {
            usage: HashMap::new(),
        }
    }

    /// Add `tokens` to the running daily total for `agent_id`.
    pub fn record_usage(&mut self, agent_id: &str, tokens: u32) {
        *self.usage.entry(agent_id.to_string()).or_insert(0) += u64::from(tokens);
    }

    /// Returns cumulative tokens used today (since last process start).
    pub fn tokens_used_today(&self, agent_id: &str) -> u64 {
        self.usage.get(agent_id).copied().unwrap_or(0)
    }

    /// Returns true when the agent has exceeded its daily budget.
    pub fn is_over_budget(&self, agent_id: &str, config: &AgentInferenceConfig) -> bool {
        self.tokens_used_today(agent_id) > config.budget_tokens_per_day
    }
}

impl Default for BudgetTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ── apply_agent_constraints ───────────────────────────────────────────────────

/// Compute the effective `InferenceTier` for a request by applying the
/// agent's constraints in order:
///
/// 1. Start from the request's tier hint (default: T1Trivial if absent).
/// 2. Clamp to `config.max_tier`.
/// 3. If the agent is over budget, downgrade one tier (floor: T1Trivial).
pub fn apply_agent_constraints(
    request: &InferenceRequest,
    config: &AgentInferenceConfig,
    budget: &BudgetTracker,
) -> InferenceTier {
    // Step 1: resolve requested tier
    let requested = request
        .tier_hint
        .clone()
        .unwrap_or(InferenceTier::T1Trivial);

    // Step 2: clamp to max allowed tier
    let clamped = if requested > config.max_tier {
        config.max_tier.clone()
    } else {
        requested
    };

    // Step 3: over-budget downgrade
    if budget.is_over_budget(&request.agent_id, config) {
        downgrade_one(clamped)
    } else {
        clamped
    }
}

/// Downgrade a tier by one step; floor is T1Trivial.
fn downgrade_one(tier: InferenceTier) -> InferenceTier {
    match tier {
        InferenceTier::T1Trivial => InferenceTier::T1Trivial,
        InferenceTier::T2Standard => InferenceTier::T1Trivial,
        InferenceTier::T3Complex => InferenceTier::T2Standard,
        InferenceTier::T4Critical => InferenceTier::T3Complex,
    }
}
