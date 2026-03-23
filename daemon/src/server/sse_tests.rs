use super::validate_peer;

#[test]
fn valid_peer_accepted() {
    assert!(validate_peer("worker-1").is_ok());
    assert!(validate_peer("node.local").is_ok());
    assert!(validate_peer("my_host-01").is_ok());
    assert!(validate_peer("abc123").is_ok());
}

#[test]
fn malicious_peer_rejected() {
    // Shell metacharacter injection must be rejected
    assert!(validate_peer(";rm -rf /").is_err());
    assert!(validate_peer("peer;bad").is_err());
    assert!(validate_peer("peer|cat /etc/passwd").is_err());
    assert!(validate_peer("$(reboot)").is_err());
    assert!(validate_peer("`id`").is_err());
    assert!(validate_peer("../../../etc/passwd").is_err());
    assert!(validate_peer("peer && evil").is_err());
}

#[test]
fn empty_peer_rejected() {
    assert!(validate_peer("").is_err());
}
