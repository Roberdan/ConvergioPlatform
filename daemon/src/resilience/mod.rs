//! Resilience primitives for the Convergio daemon.
//!
//! Implements Article XI of the Agent Constitution: "Self-recover from ANY failure."
//!
//! # Modules
//!
//! - [`circuit_breaker`]: Opens after N failures; probes on reset timeout.
//! - [`retry`]: Async exponential backoff with optional jitter.
//! - [`health`]: HealthCheck trait + HealthRegistry for component monitoring.
//! - [`reaper`]: Zero-zombie enforcement — cleans stale worktrees, branches, locks.
//! - [`checkpoint`]: Plan checkpoint persistence for restart-without-data-loss.
//! - [`notify`]: Phone notifications — ntfy.sh, Telegram, macOS.
//! - [`watchdog`]: Local LLM kernel — health monitoring + auto-recovery.

pub mod checkpoint;
pub mod circuit_breaker;
pub mod health;
pub mod notify;
pub mod notify_config;
pub mod reaper;
pub mod reaper_scan;
pub mod retry;
pub mod watchdog;

#[cfg(test)]
mod notify_tests;
#[cfg(test)]
mod watchdog_tests;
