//! Circuit breaker for external calls (HTTP, SSH, filesystem).
//!
//! Prevents cascading failures by opening after N consecutive failures
//! and allowing probe requests after a reset timeout expires.
//! Thread-safe via Arc<Mutex<CircuitBreaker>>.

use std::time::{Duration, Instant};

/// Observable states of the circuit breaker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation — requests flow through.
    Closed,
    /// Tripped — requests blocked until reset timeout expires.
    Open,
    /// Probe state — one request allowed to test recovery.
    HalfOpen,
}

/// Configuration for a circuit breaker instance.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening.
    pub failure_threshold: u32,
    /// How long to wait in Open state before probing.
    pub reset_timeout: Duration,
}

/// Circuit breaker with Closed/Open/HalfOpen state machine.
///
/// Wrap in `Arc<Mutex<CircuitBreaker>>` for shared cross-thread access.
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: CircuitState,
    failure_count: u32,
    opened_at: Option<Instant>,
}

impl CircuitBreaker {
    /// Creates a new circuit breaker in Closed state.
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: CircuitState::Closed,
            failure_count: 0,
            opened_at: None,
        }
    }

    /// Returns the current state.
    pub fn state(&self) -> CircuitState {
        self.state.clone()
    }

    /// Returns true if the request should be allowed through.
    ///
    /// - Closed: always true
    /// - Open: true only after reset_timeout (transitions to HalfOpen)
    /// - HalfOpen: true (one probe allowed; caller must call record_success/failure)
    pub fn allow_request(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::HalfOpen => true,
            CircuitState::Open => {
                if let Some(opened_at) = self.opened_at {
                    if opened_at.elapsed() >= self.config.reset_timeout {
                        self.state = CircuitState::HalfOpen;
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Records a successful operation.
    ///
    /// Resets failure counter and closes the circuit.
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.opened_at = None;
        self.state = CircuitState::Closed;
    }

    /// Records a failed operation.
    ///
    /// In Closed state: increments counter; opens if threshold reached.
    /// In HalfOpen state: probe failed; reopens the circuit.
    /// In Open state: no state change (already open).
    pub fn record_failure(&mut self) {
        match self.state {
            CircuitState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.config.failure_threshold {
                    self.trip();
                }
            }
            CircuitState::HalfOpen => {
                // Probe failed — reopen with fresh timer
                self.trip();
            }
            CircuitState::Open => {
                // Already open; update timer so backoff restarts
                self.opened_at = Some(Instant::now());
            }
        }
    }

    fn trip(&mut self) {
        self.state = CircuitState::Open;
        self.opened_at = Some(Instant::now());
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn record_success_resets_failure_count() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(60),
        });
        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        assert_eq!(cb.failure_count, 0);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn does_not_open_below_threshold() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3,
            reset_timeout: Duration::from_secs(60),
        });
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
    }
}
