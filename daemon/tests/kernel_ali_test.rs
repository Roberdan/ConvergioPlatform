// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for Ali escalation keyword classification.
// Verifies keyword_classify() behaviour through the public classify_intent() API.
// classify_intent() falls through to keyword_classify() when no model is loaded.

#![cfg(feature = "kernel")]

use claude_core::kernel::engine::{KernelConfig, KernelEngine};
use claude_core::kernel::voice_router::{classify_intent, VoiceIntent};

// Helper: build a KernelEngine with no model loaded (ensures keyword fallback).
fn unloaded_engine() -> KernelEngine {
    KernelEngine::new(KernelConfig::default())
}

// ── EscalateToAli triggers ────────────────────────────────────────────────────

/// "ali dimmi" triggers EscalateToAli (starts with "ali ").
#[test]
fn test_escalate_keyword_ali() {
    let engine = unloaded_engine();
    let intent = classify_intent("ali dimmi", &engine);
    assert!(
        matches!(intent, VoiceIntent::EscalateToAli { .. }),
        "\"ali dimmi\" must map to EscalateToAli, got: {intent:?}"
    );
}

/// "chiedi ad opus" triggers EscalateToAli (contains "opus").
#[test]
fn test_escalate_keyword_opus() {
    let engine = unloaded_engine();
    let intent = classify_intent("chiedi ad opus", &engine);
    assert!(
        matches!(intent, VoiceIntent::EscalateToAli { .. }),
        "\"chiedi ad opus\" must map to EscalateToAli, got: {intent:?}"
    );
}

/// "chiedi ad ali cosa sta succedendo" triggers EscalateToAli.
#[test]
fn test_escalate_keyword_chiedi_ad_ali() {
    let engine = unloaded_engine();
    let intent = classify_intent("chiedi ad ali cosa sta succedendo", &engine);
    assert!(
        matches!(intent, VoiceIntent::EscalateToAli { .. }),
        "\"chiedi ad ali ...\" must map to EscalateToAli, got: {intent:?}"
    );
}

/// The escalated question string is preserved verbatim.
#[test]
fn test_escalate_preserves_question_text() {
    let engine = unloaded_engine();
    let text = "ali dimmi quanti piani sono attivi";
    match classify_intent(text, &engine) {
        VoiceIntent::EscalateToAli { question } => {
            assert_eq!(question, text, "question must equal original input");
        }
        other => panic!("expected EscalateToAli, got: {other:?}"),
    }
}

// ── StatusCheck is NOT triggered by Ali keywords ─────────────────────────────

/// "stato" must map to StatusCheck, not EscalateToAli.
/// Verifies that Ali keywords are checked before "stato" in the priority chain,
/// but a plain "stato" with no Ali keyword maps correctly to StatusCheck.
#[test]
fn test_escalate_not_triggered_by_stato() {
    let engine = unloaded_engine();
    let intent = classify_intent("stato", &engine);
    assert_eq!(
        intent,
        VoiceIntent::StatusCheck,
        "\"stato\" alone must map to StatusCheck, not EscalateToAli"
    );
}

/// "status dei piani" → StatusCheck (no Ali keyword present).
#[test]
fn test_escalate_not_triggered_by_status_dei_piani() {
    let engine = unloaded_engine();
    let intent = classify_intent("status dei piani", &engine);
    assert_eq!(
        intent,
        VoiceIntent::StatusCheck,
        "\"status dei piani\" must map to StatusCheck, got: {intent:?}"
    );
}

// ── Other keywords not confused with Ali ─────────────────────────────────────

/// "costi" maps to CostQuery, not EscalateToAli.
#[test]
fn test_cost_keyword_not_escalated() {
    let engine = unloaded_engine();
    let intent = classify_intent("quanto costano i piani", &engine);
    assert_eq!(
        intent,
        VoiceIntent::CostQuery,
        "cost query must map to CostQuery, got: {intent:?}"
    );
}

/// "silenzio" maps to Mute.
#[test]
fn test_mute_keyword_not_escalated() {
    let engine = unloaded_engine();
    let intent = classify_intent("silenzio", &engine);
    assert_eq!(intent, VoiceIntent::Mute, "\"silenzio\" must map to Mute, got: {intent:?}");
}
