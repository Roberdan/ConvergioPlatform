use super::audit_chain::AuditChain;

#[test]
fn record_and_verify() {
    let chain = AuditChain::new();
    chain.record("agent-a", "read", "/data/file", "").unwrap();
    chain.record("agent-a", "write", "/data/file", "{\"key\":\"val\"}").unwrap();
    assert_eq!(chain.len(), 2);
    assert!(chain.verify().unwrap());
}

#[test]
fn chain_links_correctly() {
    let chain = AuditChain::new();
    let e1 = chain.record("agent-b", "invoke", "tool-x", "").unwrap();
    let e2 = chain.record("agent-b", "invoke", "tool-y", "").unwrap();
    assert_eq!(e2.prev_hash, e1.entry_hash);
}

#[test]
fn query_by_agent() {
    let chain = AuditChain::new();
    chain.record("agent-a", "read", "/a", "").unwrap();
    chain.record("agent-b", "write", "/b", "").unwrap();
    chain.record("agent-a", "exec", "/c", "").unwrap();
    let results = chain.query(Some("agent-a"), None);
    assert_eq!(results.len(), 2);
}

#[test]
fn query_by_action() {
    let chain = AuditChain::new();
    chain.record("agent-x", "read", "/a", "").unwrap();
    chain.record("agent-x", "write", "/b", "").unwrap();
    let results = chain.query(None, Some("write"));
    assert_eq!(results.len(), 1);
}

#[test]
fn empty_chain_verifies() {
    let chain = AuditChain::new();
    assert!(chain.verify().unwrap());
    assert!(chain.is_empty());
}

#[test]
fn params_hash_is_deterministic() {
    let chain = AuditChain::new();
    let e1 = chain.record("a", "act", "t", "same_params").unwrap();
    let chain2 = AuditChain::new();
    let e2 = chain2.record("a", "act", "t", "same_params").unwrap();
    assert_eq!(e1.params_hash, e2.params_hash);
}
