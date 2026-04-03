use super::types::{AgentBudget, SecurityError};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

/// Per-agent budget enforcer with soft/hard limits.
pub struct BudgetEnforcer {
    budgets: Mutex<HashMap<String, AgentBudget>>,
    usage: Mutex<HashMap<String, BudgetUsage>>,
}

struct BudgetUsage {
    api_calls: u64,
    tokens: u64,
    _compute_seconds: u64,
    _storage_bytes: u64,
    window_start: Instant,
}

impl BudgetEnforcer {
    pub fn new() -> Self {
        Self {
            budgets: Mutex::new(HashMap::new()),
            usage: Mutex::new(HashMap::new()),
        }
    }

    /// Configure budget for an agent.
    pub fn set_budget(&self, budget: AgentBudget) {
        let id = budget.agent_id.clone();
        if let Ok(mut budgets) = self.budgets.lock() {
            budgets.insert(id, budget);
        }
    }

    /// Record an API call. Returns error if hard limit exceeded.
    pub fn record_api_call(&self, agent_id: &str) -> Result<BudgetStatus, SecurityError> {
        self.ensure_usage(agent_id);
        if let Ok(mut usage) = self.usage.lock() {
            if let Some(u) = usage.get_mut(agent_id) {
                u.api_calls += 1;
            }
        }
        self.check_budget(agent_id)
    }

    /// Record token usage.
    pub fn record_tokens(&self, agent_id: &str, count: u64) -> Result<BudgetStatus, SecurityError> {
        self.ensure_usage(agent_id);
        if let Ok(mut usage) = self.usage.lock() {
            if let Some(u) = usage.get_mut(agent_id) {
                u.tokens += count;
            }
        }
        self.check_budget(agent_id)
    }

    /// Get current usage status for an agent.
    pub fn status(&self, agent_id: &str) -> BudgetStatus {
        let budget = match self.budgets.lock() {
            Ok(b) => b.get(agent_id).cloned().unwrap_or_default(),
            Err(e) => { tracing::warn!("budget: budgets lock poisoned: {e}"); AgentBudget::default() }
        };
        let usage = match self.usage.lock() {
            Ok(u) => u.get(agent_id).map(|v| (v.api_calls, v.tokens)),
            Err(e) => { tracing::warn!("budget: usage lock poisoned: {e}"); None }
        };
        let (calls, tokens) = usage.unwrap_or((0, 0));
        let calls_pct = if budget.max_api_calls_per_hour > 0 {
            (calls as f64 / budget.max_api_calls_per_hour as f64 * 100.0) as u8
        } else { 0 };
        let tokens_pct = if budget.max_tokens_per_day > 0 {
            (tokens as f64 / budget.max_tokens_per_day as f64 * 100.0) as u8
        } else { 0 };
        let max_pct = calls_pct.max(tokens_pct);
        BudgetStatus {
            api_calls_used: calls,
            tokens_used: tokens,
            utilization_pct: max_pct,
            soft_limit_hit: max_pct >= 80,
            hard_limit_hit: max_pct >= 100,
        }
    }

    fn ensure_usage(&self, agent_id: &str) {
        if let Ok(mut usage) = self.usage.lock() {
            usage.entry(agent_id.to_string()).or_insert(BudgetUsage {
                api_calls: 0, tokens: 0, _compute_seconds: 0, _storage_bytes: 0,
                window_start: Instant::now(),
            });
        }
    }

    /// Reset usage counters if the time window (1 hour) has expired.
    fn reset_window_if_expired(&self, agent_id: &str) {
        const WINDOW_SECS: u64 = 3600;
        if let Ok(mut usage) = self.usage.lock() {
            if let Some(u) = usage.get_mut(agent_id) {
                if u.window_start.elapsed().as_secs() >= WINDOW_SECS {
                    u.api_calls = 0;
                    u.tokens = 0;
                    u.window_start = Instant::now();
                    tracing::debug!("budget: reset window for {agent_id}");
                }
            }
        }
    }

    fn check_budget(&self, agent_id: &str) -> Result<BudgetStatus, SecurityError> {
        self.reset_window_if_expired(agent_id);
        let s = self.status(agent_id);
        if s.hard_limit_hit {
            Err(SecurityError::BudgetExceeded(format!(
                "{agent_id}: budget at {}%", s.utilization_pct
            )))
        } else {
            Ok(s)
        }
    }
}

impl Default for BudgetEnforcer {
    fn default() -> Self { Self::new() }
}

/// Budget usage status for an agent.
#[derive(Debug, Clone)]
pub struct BudgetStatus {
    pub api_calls_used: u64,
    pub tokens_used: u64,
    pub utilization_pct: u8,
    pub soft_limit_hit: bool,
    pub hard_limit_hit: bool,
}
