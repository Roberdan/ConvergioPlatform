use super::*;

#[test]
fn delegate_complete_payload_deserializes() {
    let val = json!({"task_id": "T6-03", "result": "success", "output": "all good"});
    let p: DelegateCompletePayload = serde_json::from_value(val).unwrap();
    assert_eq!(p.task_id, "T6-03");
    assert_eq!(p.result, "success");
    assert_eq!(p.output.as_deref(), Some("all good"));
}

#[test]
fn delegate_complete_payload_minimal() {
    let val = json!({"task_id": "T1-01", "result": "done"});
    let p: DelegateCompletePayload = serde_json::from_value(val).unwrap();
    assert_eq!(p.task_id, "T1-01");
    assert!(p.output.is_none());
}

#[test]
fn terminal_status_check() {
    assert!(is_terminal("done"));
    assert!(is_terminal("completed"));
    assert!(is_terminal("failed"));
    assert!(is_terminal("cancelled"));
    assert!(!is_terminal("running"));
    assert!(!is_terminal("pending"));
}

// ── Additional tests ─────────────────────────────────────────────────────────

#[test]
fn delegation_serialization_roundtrip() {
    let d = Delegation {
        task_id: "T5-02".to_string(),
        plan_id: 742,
        peer_name: "m1pro".to_string(),
        peer_addr: "http://100.64.0.2:8420".to_string(),
        status: "running".to_string(),
    };
    let json = serde_json::to_string(&d).unwrap();
    let back: Delegation = serde_json::from_str(&json).unwrap();
    assert_eq!(back.task_id, "T5-02");
    assert_eq!(back.plan_id, 742);
    assert_eq!(back.peer_name, "m1pro");
    assert_eq!(back.status, "running");
}

#[test]
fn delegate_complete_payload_with_empty_output() {
    let val = json!({"task_id": "T2-01", "result": "failed", "output": ""});
    let p: DelegateCompletePayload = serde_json::from_value(val).unwrap();
    assert_eq!(p.output.as_deref(), Some(""));
}

#[test]
fn terminal_status_edge_cases() {
    assert!(!is_terminal(""));
    assert!(!is_terminal("in_progress"));
    assert!(!is_terminal("submitted"));
    assert!(!is_terminal("blocked"));
}

#[test]
fn delegate_complete_payload_rejects_missing_fields() {
    let val = json!({"task_id": "T1-01"});
    let result = serde_json::from_value::<DelegateCompletePayload>(val);
    assert!(result.is_err());
}
