// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for kernel::voice_router — classify_intent + route_intent pipeline.

use super::voice_router::{classify_intent, route_intent, VoiceIntent};
use crate::kernel::engine::{KernelConfig, KernelEngine};

fn eng() -> KernelEngine { KernelEngine::new(KernelConfig::default()) }

// ----- classify_intent -------------------------------------------------------

#[test]
fn test_classify_status_check_italian() {
    assert_eq!(classify_intent("stato", &eng()), VoiceIntent::StatusCheck);
}

#[test]
fn test_classify_cost_query_italian() {
    assert_eq!(classify_intent("costi di oggi", &eng()), VoiceIntent::CostQuery);
}

#[test]
fn test_classify_plan_query_with_id() {
    assert_eq!(classify_intent("piano 729", &eng()), VoiceIntent::PlanQuery { plan_id: 729 });
}

#[test]
fn test_classify_restart() {
    assert_eq!(classify_intent("riavvia daemon", &eng()), VoiceIntent::Restart { target: "daemon".into() });
}

#[test]
fn test_classify_mute() {
    assert_eq!(classify_intent("silenzio", &eng()), VoiceIntent::Mute);
}

#[test]
fn test_classify_unknown_goes_to_escalate_ali() {
    let intent = classify_intent("forse domani nevica", &eng());
    assert!(matches!(intent, VoiceIntent::EscalateToAli { .. }), "got: {intent:?}");
}

#[test]
fn test_classify_case_insensitive() {
    assert_eq!(classify_intent("STATO", &eng()), VoiceIntent::StatusCheck);
    assert_eq!(classify_intent("Costi", &eng()), VoiceIntent::CostQuery);
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
    match classify_intent("crea progetto fitness con obiettivo perdere 5kg", &eng()) {
        VoiceIntent::CreateProject { name, mission } => {
            assert_eq!(name, "fitness"); assert_eq!(mission, "perdere 5kg");
        }
        other => panic!("expected CreateProject, got {other:?}"),
    }
}

#[test]
fn test_classify_create_project_english() {
    match classify_intent("create project alpha with goal ship MVP", &eng()) {
        VoiceIntent::CreateProject { name, mission } => {
            assert_eq!(name, "alpha"); assert_eq!(mission, "ship MVP");
        }
        other => panic!("expected CreateProject, got {other:?}"),
    }
}

#[test]
fn test_classify_create_project_no_mission() {
    match classify_intent("nuovo progetto marketing", &eng()) {
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
    match classify_intent("come sta il fitness?", &eng()) {
        VoiceIntent::AskOrg { name } => assert_eq!(name, "fitness"),
        other => panic!("expected AskOrg, got {other:?}"),
    }
}

#[test]
fn test_classify_ask_org_status_di() {
    match classify_intent("status di alpha", &eng()) {
        VoiceIntent::AskOrg { name } => assert_eq!(name, "alpha"),
        other => panic!("expected AskOrg, got {other:?}"),
    }
}

#[test]
fn test_classify_ask_org_update_on() {
    match classify_intent("update on marketing", &eng()) {
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
    let intent = classify_intent("fammi un report settimanale", &eng());
    assert!(matches!(intent, VoiceIntent::EscalateToAli { .. }), "got: {intent:?}");
}

#[test]
fn test_classify_analisi_keyword_escalates() {
    let intent = classify_intent("analisi dei costi mensili", &eng());
    assert!(matches!(intent, VoiceIntent::EscalateToAli { .. }), "got: {intent:?}");
}

// ----- Execution keyword classification --------------------------------------

#[test]
fn test_classify_deploy_keyword_escalates() {
    let intent = classify_intent("deploy the new version", &eng());
    assert!(matches!(intent, VoiceIntent::EscalateToAli { .. }), "got: {intent:?}");
}

#[test]
fn test_classify_esegui_keyword_escalates() {
    let intent = classify_intent("esegui il piano 42", &eng());
    assert!(matches!(intent, VoiceIntent::EscalateToAli { .. }), "got: {intent:?}");
}

// ----- CreateOrgFrom keyword classification ----------------------------------

#[test]
fn test_classify_crea_org_maps_to_create_project() {
    let engine = KernelEngine::new(KernelConfig::default());
    match classify_intent("crea org fitness", &engine) {
        VoiceIntent::CreateProject { name, .. } => assert_eq!(name, "fitness"),
        other => panic!("expected CreateProject, got {other:?}"),
    }
}

#[test]
fn test_classify_analizza_repo_maps_to_create_org_from() {
    let engine = KernelEngine::new(KernelConfig::default());
    match classify_intent("analizza repo /tmp/myrepo", &engine) {
        VoiceIntent::CreateOrgFrom { path } => assert_eq!(path, "/tmp/myrepo"),
        other => panic!("expected CreateOrgFrom, got {other:?}"),
    }
}

#[test]
fn test_classify_scan_repo_maps_to_create_org_from() {
    let engine = KernelEngine::new(KernelConfig::default());
    match classify_intent("scan repo", &engine) {
        VoiceIntent::CreateOrgFrom { path } => assert!(!path.is_empty()),
        other => panic!("expected CreateOrgFrom, got {other:?}"),
    }
}

// ----- route_intent for CreateOrgFrom ----------------------------------------

#[test]
fn test_route_create_org_from_offline() {
    let intent = VoiceIntent::CreateOrgFrom { path: "/nonexistent".into() };
    let r = route_intent(intent, "http://localhost:1");
    assert!(!r.is_empty(), "CreateOrgFrom must produce non-empty response");
}
