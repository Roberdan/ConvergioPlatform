pub mod agent_config;
pub mod classifier;
pub mod fallback;
pub mod health;
pub mod metrics;
pub mod router;
pub mod types;

#[cfg(test)]
mod agent_config_tests;

#[cfg(test)]
#[path = "fallback_tests.rs"]
mod fallback_tests;
