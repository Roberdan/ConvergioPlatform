pub mod classifier;
pub mod fallback;
pub mod health;
pub mod metrics;
pub mod router;
pub mod types;

#[cfg(test)]
#[path = "fallback_tests.rs"]
mod fallback_tests;
