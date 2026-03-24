use super::*;

#[test]
fn test_release_result_fields() {
    let r = ReleaseResult {
        workspace_id: "ws-abc".to_string(),
        pr_number: 42,
        pr_url: "https://github.com/org/repo/pull/42".to_string(),
        quality_gates_passed: true,
        merged: true,
    };
    assert_eq!(r.workspace_id, "ws-abc");
    assert_eq!(r.pr_number, 42);
    assert!(r.quality_gates_passed && r.merged);
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("ws-abc") && json.contains("42"));
}

#[tokio::test]
async fn test_release_workspace_not_found() {
    let pool = make_pool();
    let agent = make_agent(Box::new(MockConnector::new_ok()), pool);
    let result = agent.release("nonexistent", "org/repo").await;
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("workspace") || msg.contains("not found"),
        "got: {msg}"
    );
}
