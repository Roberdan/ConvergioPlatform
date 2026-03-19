use super::*;

#[test]
fn agent_cmd_claude_with_params() {
    let mut qs = HashMap::new();
    qs.insert("task_id".into(), "T1-02".into());
    qs.insert("wave_id".into(), "W1".into());
    let cmd = build_agent_command("claude", "671", &qs);
    assert!(cmd.contains("plan 671") && cmd.contains("task T1-02"));
}

#[test]
fn agent_cmd_custom_cli() {
    let cmd = build_agent_command("my-agent", "42", &HashMap::new());
    assert!(cmd.contains("my-agent --plan 42"));
}

#[test]
fn stage_event_fields() {
    let ev = stage("connecting", "worker-1", "SSH handshake");
    assert_eq!(ev["stage"], "connecting");
    assert_eq!(ev["peer"], "worker-1");
}

#[test]
fn cancel_delegation_lifecycle() {
    let del_id = generate_delegation_id("999", "test-peer");
    assert!(!cancel_delegation(&del_id));
    let cancelled = Arc::new(AtomicBool::new(false));
    active_delegations().lock().unwrap().insert(
        del_id.clone(),
        ActiveDelegation {
            cancelled: Arc::clone(&cancelled),
        },
    );
    assert!(cancel_delegation(&del_id));
    assert!(cancelled.load(Ordering::Acquire));
    assert!(list_active_delegations().is_empty());
}
