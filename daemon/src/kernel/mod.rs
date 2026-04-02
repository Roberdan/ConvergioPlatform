// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Kernel module — model-agnostic health monitoring and situational classification.
// Replaces the Ollama-based watchdog. Use `cvg kernel` for all health/monitoring operations.

pub mod audio;
pub mod audio_routing;
pub mod engine;
pub mod engine_context;
pub mod engine_tool_loop;
pub mod tools;
pub mod tools_legacy;
pub mod cloud_escalation;
pub mod monitor;
pub mod monitor_checks;
pub mod monitor_ext;
pub mod monitor_peer_tracker;
#[cfg(test)]
mod monitor_checks_tests;
pub mod recover;
pub mod recover_escalation;
pub mod reports;
pub mod stt;
pub mod telegram;
pub mod telegram_conv;
pub mod telegram_poll;
pub mod telegram_voice;
pub mod tts;
pub mod tts_ext;
pub mod tts_templates;
pub mod verify;
pub mod verify_checks;
pub mod verify_hardening;
pub mod voice_route_project;
pub mod voice_router;
pub mod org_router;
pub mod voice_router_helpers;
pub mod voice_routes;

#[cfg(feature = "kernel")]
pub mod api;
#[cfg(feature = "kernel")]
pub mod api_ask;
#[cfg(feature = "kernel")]
pub mod api_listen;

#[cfg(test)]
mod api_ask_tests;
#[cfg(test)]
mod engine_tests;
#[cfg(test)]
mod monitor_peer_tracker_tests;
#[cfg(test)]
mod monitor_tests;
#[cfg(test)]
mod monitor_tests_db;
#[cfg(test)]
mod recover_tests;
#[cfg(test)]
mod recover_escalation_tests;
#[cfg(test)]
mod stt_tests;
#[cfg(test)]
mod voice_router_tests;
#[cfg(test)]
mod telegram_poll_tests;
#[cfg(test)]
mod verify_hardening_tests;
#[cfg(test)]
mod cloud_escalation_tests;
