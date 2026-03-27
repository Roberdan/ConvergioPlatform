// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Kernel module — model-agnostic health monitoring and situational classification.
// Replaces the Ollama-based watchdog. Use `cvg kernel` for all health/monitoring operations.

pub mod engine;
pub mod monitor;

#[cfg(feature = "kernel")]
pub mod api;

#[cfg(test)]
mod monitor_tests;
#[cfg(test)]
mod engine_tests;
