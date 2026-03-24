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
