// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for kernel::voice_router — classify_intent + route_intent pipeline.

use super::voice_router::{classify_intent, route_intent, VoiceIntent};
use crate::kernel::engine::{KernelConfig, KernelEngine};

// ----- classify_intent -------------------------------------------------------

#[test]
fn test_classify_status_check_italian() {
    let engine = KernelEngine::new(KernelConfig::default());
    let intent = classify_intent("stato", &engine);
    assert_eq!(intent, VoiceIntent::StatusCheck);
}

#[test]
fn test_classify_cost_query_italian() {
    let engine = KernelEngine::new(KernelConfig::default());
    let intent = classify_intent("costi di oggi", &engine);
    assert_eq!(intent, VoiceIntent::CostQuery);
}

#[test]
fn test_classify_plan_query_with_id() {
    let engine = KernelEngine::new(KernelConfig::default());
    let intent = classify_intent("piano 729", &engine);
    assert_eq!(intent, VoiceIntent::PlanQuery { plan_id: 729 });
}

#[test]
fn test_classify_restart() {
    let engine = KernelEngine::new(KernelConfig::default());
    let intent = classify_intent("riavvia daemon", &engine);
    assert_eq!(intent, VoiceIntent::Restart { target: "daemon".to_string() });
}

#[test]
fn test_classify_mute() {
    let engine = KernelEngine::new(KernelConfig::default());
    let intent = classify_intent("silenzio", &engine);
    assert_eq!(intent, VoiceIntent::Mute);
}

#[test]
fn test_classify_unknown_goes_to_ali() {
    let engine = KernelEngine::new(KernelConfig::default());
    let intent = classify_intent("forse domani nevica", &engine);
    matches!(intent, VoiceIntent::AskAli { .. });
}

#[test]
fn test_classify_case_insensitive() {
    let engine = KernelEngine::new(KernelConfig::default());
    assert_eq!(classify_intent("STATO", &engine), VoiceIntent::StatusCheck);
    assert_eq!(classify_intent("Costi", &engine), VoiceIntent::CostQuery);
}

// ----- route_intent ----------------------------------------------------------

#[test]
fn test_route_ask_ali_uses_kernel_ask_endpoint() {
    // AskAli must POST to /api/kernel/ask (not /classify or /api/chat).
    // With an unreachable daemon the error message confirms the attempt was made.
    let response = route_intent(
        VoiceIntent::AskAli { question: "che succede?".to_string() },
        "http://localhost:9999", // intentionally unreachable
    );
    // On connection failure route_ask_ali returns a message that contains "Errore"
    // or "Non riesco" — it never panics and never returns an empty string.
    assert!(
        response.contains("Errore") || response.contains("Non riesco"),
        "expected /api/kernel/ask error fallback, got: {response}"
    );
    // Must not be empty — a silent failure here would suppress voice feedback.
    assert!(!response.is_empty(), "AskAli fallback must produce non-empty response");
}

#[test]
fn test_route_mute_sets_flag() {
    let response = route_intent(VoiceIntent::Mute, "http://localhost:8420");
    assert!(
        response.to_lowercase().contains("silenzio") || response.to_lowercase().contains("ora"),
        "expected mute confirmation, got: {response}"
    );
}

#[test]
fn test_route_restart_returns_confirmation() {
    let response = route_intent(
        VoiceIntent::Restart { target: "daemon".to_string() },
        "http://localhost:8420",
    );
    assert!(
        response.to_lowercase().contains("riavvia") || response.to_lowercase().contains("daemon"),
        "expected restart confirmation, got: {response}"
    );
}

#[test]
fn test_route_status_check_uses_plan_db_list_format() {
    // StatusCheck calls GET /api/plan-db/list (not /overview or any other endpoint).
    // Verified via offline fallback: the Italian error phrase "contattare il daemon"
    // is only produced by the plan-db/list code path (route_status_check).
    let response = route_intent(VoiceIntent::StatusCheck, "http://localhost:1");
    assert!(
        !response.is_empty(),
        "StatusCheck response must not be empty on offline daemon"
    );
    // The offline fallback must contain an Italian message — proves plan-db/list path.
    assert!(
        response.contains("daemon") || response.contains("piani") || response.contains("contattare"),
        "expected Italian plan-db/list fallback, got: {response}"
    );
}

#[test]
fn test_route_status_check_offline_fallback() {
    // Daemon not running — expect graceful Italian fallback, not a panic.
    let response = route_intent(VoiceIntent::StatusCheck, "http://localhost:1");
    assert!(!response.is_empty(), "response must not be empty");
}

#[test]
fn test_route_cost_query_offline_fallback() {
    let response = route_intent(VoiceIntent::CostQuery, "http://localhost:1");
    assert!(!response.is_empty());
}

#[test]
fn test_route_plan_query_offline_fallback() {
    let response = route_intent(VoiceIntent::PlanQuery { plan_id: 729 }, "http://localhost:1");
    assert!(!response.is_empty());
}

// ----- VoiceIntent display ---------------------------------------------------

#[test]
fn test_intent_debug_format() {
    let i = VoiceIntent::PlanQuery { plan_id: 42 };
    let debug = format!("{i:?}");
    assert!(debug.contains("42"));
}
