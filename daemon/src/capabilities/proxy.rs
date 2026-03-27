use super::types::CapabilityError;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Proxy layer for MCP tool calls with rate limiting and circuit breaking.
pub struct CapabilityProxy {
    rate_limits: Mutex<HashMap<String, RateState>>,
    circuits: Mutex<HashMap<String, CircuitState>>,
    max_per_minute: u32,
    failure_threshold: u32,
    circuit_timeout: Duration,
}

struct RateState {
    count: u32,
    window_start: Instant,
}

struct CircuitState {
    failures: u32,
    state: CircuitStatus,
    last_failure: Instant,
}

#[derive(Clone, Copy, PartialEq)]
enum CircuitStatus {
    Closed,
    Open,
    HalfOpen,
}

impl CapabilityProxy {
    pub fn new() -> Self {
        Self {
            rate_limits: Mutex::new(HashMap::new()),
            circuits: Mutex::new(HashMap::new()),
            max_per_minute: 60,
            failure_threshold: 5,
            circuit_timeout: Duration::from_secs(30),
        }
    }

    /// Check rate limit and circuit breaker before invocation.
    pub fn pre_invoke(&self, agent_id: &str, tool_name: &str) -> Result<(), CapabilityError> {
        self.check_circuit(tool_name)?;
        self.check_rate_limit(agent_id, tool_name)?;
        Ok(())
    }

    /// Record successful invocation.
    pub fn record_success(&self, tool_name: &str) {
        if let Ok(mut circuits) = self.circuits.lock() {
            if let Some(cs) = circuits.get_mut(tool_name) {
                cs.state = CircuitStatus::Closed;
                cs.failures = 0;
            }
        }
    }

    /// Record failed invocation. Opens circuit after threshold.
    pub fn record_failure(&self, tool_name: &str) {
        if let Ok(mut circuits) = self.circuits.lock() {
            let cs = circuits
                .entry(tool_name.to_string())
                .or_insert(CircuitState {
                    failures: 0,
                    state: CircuitStatus::Closed,
                    last_failure: Instant::now(),
                });
            cs.failures += 1;
            cs.last_failure = Instant::now();
            if cs.failures >= self.failure_threshold {
                cs.state = CircuitStatus::Open;
            }
        }
    }

    fn check_circuit(&self, tool_name: &str) -> Result<(), CapabilityError> {
        if let Ok(mut circuits) = self.circuits.lock() {
            if let Some(cs) = circuits.get_mut(tool_name) {
                match cs.state {
                    CircuitStatus::Open => {
                        if cs.last_failure.elapsed() > self.circuit_timeout {
                            cs.state = CircuitStatus::HalfOpen;
                            return Ok(());
                        }
                        return Err(CapabilityError::CircuitOpen(tool_name.to_string()));
                    }
                    CircuitStatus::HalfOpen | CircuitStatus::Closed => {}
                }
            }
        }
        Ok(())
    }

    fn check_rate_limit(&self, agent_id: &str, tool_name: &str) -> Result<(), CapabilityError> {
        let key = format!("{agent_id}:{tool_name}");
        if let Ok(mut limits) = self.rate_limits.lock() {
            let now = Instant::now();
            let state = limits.entry(key.clone()).or_insert(RateState {
                count: 0,
                window_start: now,
            });
            if now.duration_since(state.window_start) > Duration::from_secs(60) {
                state.count = 0;
                state.window_start = now;
            }
            state.count += 1;
            if state.count > self.max_per_minute {
                return Err(CapabilityError::RateLimited(format!(
                    "{agent_id} exceeded {}/min for {tool_name}",
                    self.max_per_minute
                )));
            }
        }
        Ok(())
    }
}

impl Default for CapabilityProxy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_allows_within_threshold() {
        let proxy = CapabilityProxy::new();
        for _ in 0..60 {
            proxy.pre_invoke("agent-a", "tool-x").unwrap();
        }
    }

    #[test]
    fn rate_limit_blocks_over_threshold() {
        let proxy = CapabilityProxy::new();
        for _ in 0..60 {
            proxy.pre_invoke("agent-b", "tool-y").unwrap();
        }
        let err = proxy.pre_invoke("agent-b", "tool-y").unwrap_err();
        assert!(matches!(err, CapabilityError::RateLimited(_)));
    }

    #[test]
    fn circuit_opens_after_failures() {
        let proxy = CapabilityProxy::new();
        for _ in 0..5 {
            proxy.record_failure("flaky-tool");
        }
        let err = proxy.pre_invoke("agent-c", "flaky-tool").unwrap_err();
        assert!(matches!(err, CapabilityError::CircuitOpen(_)));
    }

    #[test]
    fn circuit_success_resets() {
        let proxy = CapabilityProxy::new();
        proxy.record_failure("tool-z");
        proxy.record_failure("tool-z");
        proxy.record_success("tool-z");
        proxy.pre_invoke("agent-d", "tool-z").unwrap();
    }
}
