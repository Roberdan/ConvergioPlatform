// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Kernel module — model-agnostic health monitoring and situational classification.
// Replaces the Ollama-based watchdog. Use `cvg kernel` for all health/monitoring operations.

pub mod audio;
pub mod engine;
pub mod monitor;
pub mod recover;
pub mod reports;
pub mod stt;
pub mod telegram;
pub mod telegram_poll;
pub mod telegram_voice;
pub mod tts;
pub mod verify;
pub mod voice_router;

#[cfg(feature = "kernel")]
pub mod api;

#[cfg(test)]
mod engine_tests;
#[cfg(test)]
mod monitor_tests;
#[cfg(test)]
mod recover_tests;
#[cfg(test)]
mod stt_tests;
#[cfg(test)]
mod voice_router_tests;
#[cfg(test)]
mod telegram_poll_tests;
