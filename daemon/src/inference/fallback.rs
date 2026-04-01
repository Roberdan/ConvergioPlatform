/// Configurable fallback chains for inference routing (F-06).
///
/// Each tier has an ordered list of model names tried in sequence.
/// The executor stops after `max_attempts` regardless of chain length.
/// Every fallback attempt is logged with the reason for the previous failure.
use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;
use tracing::{info, warn};

use super::types::InferenceTier;

// ---------------------------------------------------------------------------
// FallbackChain
// ---------------------------------------------------------------------------

/// Ordered list of model names for a single tier.
#[derive(Debug, Clone)]
pub struct FallbackChain {
    models: Vec<String>,
}

impl FallbackChain {
    pub fn new(models: Vec<String>) -> Self {
        Self { models }
    }

    pub fn models(&self) -> &[String] {
        &self.models
    }
}

// ---------------------------------------------------------------------------
// FallbackConfig
// ---------------------------------------------------------------------------

/// Raw YAML structure for deserialization.
#[derive(Debug, Deserialize)]
struct FallbackConfigRaw {
    max_attempts: usize,
    chains: HashMap<String, Vec<String>>,
}

/// Holds per-tier fallback chains and the global max attempt count.
#[derive(Debug, Clone)]
pub struct FallbackConfig {
    chains: HashMap<String, FallbackChain>,
    max_attempts: usize,
}

impl FallbackConfig {
    /// Load config from a YAML file.
    ///
    /// Returns `Err` on I/O failure or malformed YAML.
    pub fn load(path: &Path) -> Result<Self, String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
        let parsed: FallbackConfigRaw =
            serde_yaml::from_str(&raw).map_err(|e| format!("YAML parse error: {}", e))?;

        let chains = parsed
            .chains
            .into_iter()
            .map(|(k, v)| (k, FallbackChain::new(v)))
            .collect();

        Ok(Self {
            chains,
            max_attempts: parsed.max_attempts,
        })
    }

    /// Build from the config.toml `[inference.fallback]` section.
    pub fn from_config(cfg: &crate::config::InferenceFallbackConfig) -> Self {
        let mut chains = HashMap::new();
        chains.insert("t1".into(), FallbackChain::new(cfg.t1.clone()));
        chains.insert("t2".into(), FallbackChain::new(cfg.t2.clone()));
        chains.insert("t3".into(), FallbackChain::new(cfg.t3.clone()));
        chains.insert("t4".into(), FallbackChain::new(cfg.t4.clone()));
        Self {
            chains,
            max_attempts: cfg.max_attempts,
        }
    }

    /// Hardcoded default chains matching the platform spec:
    /// T1: local -> haiku -> sonnet
    /// T2: haiku -> local -> sonnet
    /// T3: sonnet -> opus
    /// T4: opus -> sonnet (with warning on use)
    pub fn default_chains() -> Self {
        let mut chains = HashMap::new();
        chains.insert(
            "t1".into(),
            FallbackChain::new(vec!["local".into(), "haiku".into(), "sonnet".into()]),
        );
        chains.insert(
            "t2".into(),
            FallbackChain::new(vec!["haiku".into(), "local".into(), "sonnet".into()]),
        );
        chains.insert(
            "t3".into(),
            FallbackChain::new(vec!["sonnet".into(), "opus".into()]),
        );
        chains.insert(
            "t4".into(),
            FallbackChain::new(vec!["opus".into(), "sonnet".into()]),
        );
        Self {
            chains,
            max_attempts: 3,
        }
    }

    /// Return the fallback chain for a given tier. Falls back to an empty slice if unknown.
    pub fn chain_for(&self, tier: &InferenceTier) -> &[String] {
        let key = tier_key(tier);
        self.chains
            .get(key)
            .map(|c| c.models())
            .unwrap_or_default()
    }

    pub fn max_attempts(&self) -> usize {
        self.max_attempts
    }
}

fn tier_key(tier: &InferenceTier) -> &'static str {
    match tier {
        InferenceTier::T1Trivial => "t1",
        InferenceTier::T2Standard => "t2",
        InferenceTier::T3Complex => "t3",
        InferenceTier::T4Critical => "t4",
    }
}

// ---------------------------------------------------------------------------
// FallbackResult
// ---------------------------------------------------------------------------

/// Outcome of a successful fallback execution.
#[derive(Debug, Clone)]
pub struct FallbackResult {
    /// Name of the model that finally succeeded.
    pub model_used: String,
    /// 1-based index of the attempt that succeeded.
    pub attempt: usize,
    /// Reason from the previous failure, if any fallback occurred.
    pub fallback_reason: Option<String>,
}

// ---------------------------------------------------------------------------
// FallbackExecutor
// ---------------------------------------------------------------------------

/// Stateless executor that tries each model in the chain in order.
pub struct FallbackExecutor;

impl FallbackExecutor {
    /// Try each model in `chain`, calling `attempt_fn(model_name)` up to `max_attempts`.
    ///
    /// Returns `Ok(FallbackResult)` on first success or `Err` if all attempts fail.
    /// Logs each fallback with the failure reason (INFO for fallback, WARN for T4 demotion).
    pub fn execute_with_fallback<F, T>(
        chain: &[String],
        max_attempts: usize,
        mut attempt_fn: F,
    ) -> Result<FallbackResult, String>
    where
        F: FnMut(&str) -> Result<T, String>,
    {
        if chain.is_empty() {
            return Err("empty chain: no models to try".into());
        }

        let mut last_reason: Option<String> = None;

        for (idx, model) in chain.iter().enumerate() {
            if idx >= max_attempts {
                break;
            }

            // T4 demotion warning: when we fall to the second model in the t4 chain
            if idx > 0 {
                let reason = last_reason.as_deref().unwrap_or("unknown");
                if model == "sonnet" && chain.first().map(|s| s.as_str()) == Some("opus") {
                    warn!(
                        model = %model,
                        attempt = idx + 1,
                        reason = reason,
                        "T4 demotion: falling back from opus to sonnet"
                    );
                } else {
                    info!(
                        model = %model,
                        attempt = idx + 1,
                        reason = reason,
                        "inference fallback triggered"
                    );
                }
            }

            match attempt_fn(model) {
                Ok(_output) => {
                    return Ok(FallbackResult {
                        model_used: model.clone(),
                        attempt: idx + 1,
                        fallback_reason: last_reason,
                    });
                }
                Err(reason) => {
                    info!(
                        model = %model,
                        attempt = idx + 1,
                        reason = %reason,
                        "inference attempt failed"
                    );
                    last_reason = Some(reason);
                }
            }
        }

        Err(format!(
            "all {} fallback attempts exhausted: {}",
            max_attempts,
            last_reason.unwrap_or_default()
        ))
    }
}
