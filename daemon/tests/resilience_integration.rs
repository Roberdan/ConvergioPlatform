//! Integration tests for the resilience module (circuit breaker + retry).
//!
//! TDD: these tests are written BEFORE implementation to define required behavior.

use std::sync::{Arc, Mutex};
use std::time::Duration;

// These imports will fail until the module is implemented (RED phase).
use convergio_core::resilience::{
    circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState},
    health::{ComponentHealth, HealthCheck, HealthStatus},
    retry::{retry_with_backoff, RetryConfig},
};

// ---------------------------------------------------------------------------
// Circuit breaker tests
// ---------------------------------------------------------------------------

#[test]
fn circuit_breaker_starts_closed() {
    let cb = CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 3,
        reset_timeout: Duration::from_secs(5),
    });
    assert_eq!(cb.state(), CircuitState::Closed);
}

#[test]
fn circuit_breaker_opens_after_threshold_failures() {
    let cb = Arc::new(Mutex::new(CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 3,
        reset_timeout: Duration::from_secs(60),
    })));

    // Simulate N failures
    for _ in 0..3 {
        cb.lock().unwrap().record_failure();
    }

    assert_eq!(
        cb.lock().unwrap().state(),
        CircuitState::Open,
        "circuit should open after threshold failures"
    );
}

#[test]
fn circuit_breaker_allows_call_when_closed() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 3,
        reset_timeout: Duration::from_secs(60),
    });
    assert!(cb.allow_request(), "closed circuit should allow requests");
}

#[test]
fn circuit_breaker_blocks_call_when_open() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 2,
        reset_timeout: Duration::from_secs(60),
    });

    cb.record_failure();
    cb.record_failure();

    assert!(
        !cb.allow_request(),
        "open circuit should block requests"
    );
}

#[test]
fn circuit_breaker_transitions_to_half_open_after_timeout() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 1,
        reset_timeout: Duration::from_millis(10),
    });

    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);

    // Wait for reset timeout to expire
    std::thread::sleep(Duration::from_millis(20));

    // allow_request() should probe and transition to HalfOpen
    assert!(cb.allow_request(), "half-open circuit should allow probe request");
    assert_eq!(cb.state(), CircuitState::HalfOpen);
}

#[test]
fn circuit_breaker_closes_after_success_in_half_open() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 1,
        reset_timeout: Duration::from_millis(10),
    });

    cb.record_failure();
    std::thread::sleep(Duration::from_millis(20));

    // Probe succeeds
    cb.allow_request();
    cb.record_success();

    assert_eq!(
        cb.state(),
        CircuitState::Closed,
        "circuit should close after successful probe"
    );
}

#[test]
fn circuit_breaker_reopens_on_failure_in_half_open() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 1,
        reset_timeout: Duration::from_millis(10),
    });

    cb.record_failure();
    std::thread::sleep(Duration::from_millis(20));

    cb.allow_request(); // moves to HalfOpen
    cb.record_failure(); // probe fails → back to Open

    assert_eq!(
        cb.state(),
        CircuitState::Open,
        "failed probe should reopen the circuit"
    );
}

#[test]
fn circuit_breaker_is_thread_safe() {
    let cb = Arc::new(Mutex::new(CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 10,
        reset_timeout: Duration::from_secs(60),
    })));

    let handles: Vec<_> = (0..5)
        .map(|_| {
            let cb = Arc::clone(&cb);
            std::thread::spawn(move || {
                cb.lock().unwrap().record_failure();
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    // 5 failures, threshold is 10, should still be Closed
    assert_eq!(cb.lock().unwrap().state(), CircuitState::Closed);
}

// ---------------------------------------------------------------------------
// Retry tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn retry_succeeds_on_first_attempt() {
    let count = Arc::new(Mutex::new(0u32));
    let c = Arc::clone(&count);

    let result = retry_with_backoff(
        || {
            *c.lock().unwrap() += 1;
            async { Ok::<i32, String>(42) }
        },
        RetryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            backoff_factor: 2.0,
            jitter: false,
        },
    )
    .await;

    assert_eq!(result.unwrap(), 42);
    assert_eq!(*count.lock().unwrap(), 1);
}

#[tokio::test]
async fn retry_succeeds_after_transient_failure() {
    let count = Arc::new(Mutex::new(0u32));
    let c = Arc::clone(&count);

    let result = retry_with_backoff(
        || {
            let mut n = c.lock().unwrap();
            *n += 1;
            let attempt = *n;
            drop(n);
            async move {
                if attempt < 3 {
                    Err("transient".to_string())
                } else {
                    Ok::<i32, String>(99)
                }
            }
        },
        RetryConfig {
            max_retries: 5,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            backoff_factor: 2.0,
            jitter: false,
        },
    )
    .await;

    assert_eq!(result.unwrap(), 99);
    assert_eq!(*count.lock().unwrap(), 3);
}

#[tokio::test]
async fn retry_exhausts_max_retries() {
    let count = Arc::new(Mutex::new(0u32));
    let c = Arc::clone(&count);

    let result: Result<i32, String> = retry_with_backoff(
        || {
            *c.lock().unwrap() += 1;
            async { Err("permanent".to_string()) }
        },
        RetryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(5),
            backoff_factor: 2.0,
            jitter: false,
        },
    )
    .await;

    assert!(result.is_err());
    // max_retries=3 means initial attempt + 3 retries = 4 total
    assert_eq!(*count.lock().unwrap(), 4);
}

#[tokio::test]
async fn retry_with_jitter_does_not_exceed_max_delay() {
    // Jitter should not cause delay to exceed max_delay significantly.
    // This test just confirms it completes without hanging.
    let result: Result<i32, String> = retry_with_backoff(
        || async { Ok(1) },
        RetryConfig {
            max_retries: 2,
            initial_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(5),
            backoff_factor: 2.0,
            jitter: true,
        },
    )
    .await;

    assert_eq!(result.unwrap(), 1);
}

// ---------------------------------------------------------------------------
// Health check tests
// ---------------------------------------------------------------------------

struct MockComponent {
    status: HealthStatus,
}

impl HealthCheck for MockComponent {
    fn name(&self) -> &str {
        "mock-component"
    }

    fn check(&self) -> ComponentHealth {
        ComponentHealth {
            name: self.name().to_string(),
            status: self.status.clone(),
            message: None,
        }
    }
}

#[test]
fn health_check_returns_healthy_status() {
    let component = MockComponent {
        status: HealthStatus::Healthy,
    };
    let health = component.check();
    assert_eq!(health.status, HealthStatus::Healthy);
    assert_eq!(health.name, "mock-component");
}

#[test]
fn health_check_returns_degraded_status() {
    let component = MockComponent {
        status: HealthStatus::Degraded,
    };
    let health = component.check();
    assert_eq!(health.status, HealthStatus::Degraded);
}

#[test]
fn health_check_returns_unhealthy_status() {
    let component = MockComponent {
        status: HealthStatus::Unhealthy,
    };
    let health = component.check();
    assert_eq!(health.status, HealthStatus::Unhealthy);
}

// ---------------------------------------------------------------------------
// Integration: circuit breaker + retry working together
// ---------------------------------------------------------------------------

#[tokio::test]
async fn circuit_breaker_opens_after_n_failures_via_retry() {
    let cb = Arc::new(Mutex::new(CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 3,
        reset_timeout: Duration::from_secs(60),
    })));

    let failure_count = Arc::new(Mutex::new(0u32));

    // Drive 3 failures through retry (each retry call drives 1 op to the cb)
    for _ in 0..3 {
        let fc = Arc::clone(&failure_count);
        let cb_ref = Arc::clone(&cb);
        let _result: Result<(), String> = retry_with_backoff(
            || {
                *fc.lock().unwrap() += 1;
                cb_ref.lock().unwrap().record_failure();
                async { Err("fail".to_string()) }
            },
            RetryConfig {
                max_retries: 0, // no retries — each call drives exactly 1 failure
                initial_delay: Duration::from_millis(1),
                max_delay: Duration::from_millis(1),
                backoff_factor: 1.0,
                jitter: false,
            },
        )
        .await;
    }

    assert_eq!(
        cb.lock().unwrap().state(),
        CircuitState::Open,
        "circuit should be open after 3 failures"
    );
    assert!(
        !cb.lock().unwrap().allow_request(),
        "open circuit must block subsequent requests"
    );
}
