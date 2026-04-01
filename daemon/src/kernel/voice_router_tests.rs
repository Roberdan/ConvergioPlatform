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
fn test_classify_unknown_goes_to_escalate_ali() {
    let engine = KernelEngine::new(KernelConfig::default());
    let intent = classify_intent("forse domani nevica", &engine);
    assert!(
        matches!(intent, VoiceIntent::EscalateToAli { .. }),
        "unrecognised input must escalate to Ali, got: {intent:?}"
    );
}

#[test]
fn test_classify_case_insensitive() {
    let engine = KernelEngine::new(KernelConfig::default());
    assert_eq!(classify_intent("STATO", &engine), VoiceIntent::StatusCheck);
    assert_eq!(classify_intent("Costi", &engine), VoiceIntent::CostQuery);
}

// ----- route_intent ----------------------------------------------------------

#[test]
fn test_route_escalate_to_ali_offline() {
    let r = route_intent(
        VoiceIntent::EscalateToAli { question: "che succede?".into() },
        "http://localhost:9999",
    );
    assert!(!r.is_empty(), "EscalateToAli fallback must produce non-empty response");
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
fn test_route_status_check_offline_fallback() {
    // Daemon not running — graceful Italian fallback via /api/plan-db/list path.
    let response = route_intent(VoiceIntent::StatusCheck, "http://localhost:1");
    assert!(!response.is_empty(), "StatusCheck must not be empty on offline daemon");
    assert!(
        response.contains("daemon") || response.contains("piani") || response.contains("contattare"),
        "expected Italian plan-db/list fallback, got: {response}"
    );
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

// ----- CreateProject keyword classification ----------------------------------

#[test]
fn test_classify_create_project_italian() {
    let engine = KernelEngine::new(KernelConfig::default());
    match classify_intent("crea progetto fitness con obiettivo perdere 5kg", &engine) {
        VoiceIntent::CreateProject { name, mission } => {
            assert_eq!(name, "fitness");
            assert_eq!(mission, "perdere 5kg");
        }
        other => panic!("expected CreateProject, got {other:?}"),
    }
}

#[test]
fn test_classify_create_project_english() {
    let engine = KernelEngine::new(KernelConfig::default());
    match classify_intent("create project alpha with goal ship MVP", &engine) {
        VoiceIntent::CreateProject { name, mission } => {
            assert_eq!(name, "alpha");
            assert_eq!(mission, "ship MVP");
        }
        other => panic!("expected CreateProject, got {other:?}"),
    }
}

#[test]
fn test_classify_create_project_no_mission() {
    let engine = KernelEngine::new(KernelConfig::default());
    match classify_intent("nuovo progetto marketing", &engine) {
        VoiceIntent::CreateProject { name, mission } => {
            assert_eq!(name, "marketing");
            assert!(mission.is_empty(), "expected empty mission, got: {mission}");
        }
        other => panic!("expected CreateProject, got {other:?}"),
    }
}

// ----- AskOrg keyword classification -----------------------------------------

#[test]
fn test_classify_ask_org_come_sta() {
    let engine = KernelEngine::new(KernelConfig::default());
    match classify_intent("come sta il fitness?", &engine) {
        VoiceIntent::AskOrg { name } => assert_eq!(name, "fitness"),
        other => panic!("expected AskOrg, got {other:?}"),
    }
}

#[test]
fn test_classify_ask_org_status_di() {
    let engine = KernelEngine::new(KernelConfig::default());
    match classify_intent("status di alpha", &engine) {
        VoiceIntent::AskOrg { name } => assert_eq!(name, "alpha"),
        other => panic!("expected AskOrg, got {other:?}"),
    }
}

#[test]
fn test_classify_ask_org_update_on() {
    let engine = KernelEngine::new(KernelConfig::default());
    match classify_intent("update on marketing", &engine) {
        VoiceIntent::AskOrg { name } => assert_eq!(name, "marketing"),
        other => panic!("expected AskOrg, got {other:?}"),
    }
}

// ----- route_intent for new intents ------------------------------------------

#[test]
fn test_route_create_project_offline() {
    let intent = VoiceIntent::CreateProject { name: "fitness".into(), mission: "perdere 5kg".into() };
    let r = route_intent(intent, "http://localhost:1");
    assert!(!r.is_empty(), "CreateProject must produce non-empty response");
}

#[test]
fn test_route_ask_org_offline() {
    let r = route_intent(VoiceIntent::AskOrg { name: "fitness".into() }, "http://localhost:1");
    assert!(!r.is_empty(), "AskOrg must produce non-empty response");
}

// ----- Report/analysis keyword classification -------------------------------

#[test]
fn test_classify_report_keyword_escalates() {
    let engine = KernelEngine::new(KernelConfig::default());
    let intent = classify_intent("fammi un report settimanale", &engine);
    assert!(
        matches!(intent, VoiceIntent::EscalateToAli { .. }),
        "report keyword must escalate to Ali, got: {intent:?}"
    );
}

#[test]
fn test_classify_analisi_keyword_escalates() {
    let engine = KernelEngine::new(KernelConfig::default());
    let intent = classify_intent("analisi dei costi mensili", &engine);
    assert!(
        matches!(intent, VoiceIntent::EscalateToAli { .. }),
        "analisi keyword must escalate to Ali, got: {intent:?}"
    );
}

// ----- Execution keyword classification --------------------------------------

#[test]
fn test_classify_deploy_keyword_escalates() {
    let engine = KernelEngine::new(KernelConfig::default());
    let intent = classify_intent("deploy the new version", &engine);
    assert!(
        matches!(intent, VoiceIntent::EscalateToAli { .. }),
        "deploy keyword must escalate to Ali, got: {intent:?}"
    );
}

#[test]
fn test_classify_esegui_keyword_escalates() {
    let engine = KernelEngine::new(KernelConfig::default());
    let intent = classify_intent("esegui il piano 42", &engine);
    assert!(
        matches!(intent, VoiceIntent::EscalateToAli { .. }),
        "esegui keyword must escalate to Ali, got: {intent:?}"
    );
}

// ----- VoiceIntent display ---------------------------------------------------

#[test]
fn test_intent_debug_format() {
    let i = VoiceIntent::PlanQuery { plan_id: 42 };
    let debug = format!("{i:?}");
    assert!(debug.contains("42"));
}
