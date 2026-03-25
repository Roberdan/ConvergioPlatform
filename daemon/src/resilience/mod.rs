//! Resilience primitives for the Convergio daemon.
//!
//! Implements Article XI of the Agent Constitution: "Self-recover from ANY failure."
//!
//! # Modules
//!
//! - [`circuit_breaker`]: Opens after N failures; probes on reset timeout.
//! - [`retry`]: Async exponential backoff with optional jitter.
//! - [`health`]: HealthCheck trait + HealthRegistry for component monitoring.
//!
//! # Quick start
//!
//! ```ignore
//! use std::sync::{Arc, Mutex};
//! use crate::resilience::{
//!     circuit_breaker::{CircuitBreaker, CircuitBreakerConfig},
//!     retry::{retry_with_backoff, RetryConfig},
//! };
//!
//! let cb = Arc::new(Mutex::new(CircuitBreaker::new(CircuitBreakerConfig {
//!     failure_threshold: 5,
//!     reset_timeout: std::time::Duration::from_secs(30),
//! })));
//!
//! let result = retry_with_backoff(
//!     || async { external_call().await },
//!     RetryConfig::default(),
//! ).await;
//! ```

pub mod circuit_breaker;
pub mod health;
pub mod retry;
