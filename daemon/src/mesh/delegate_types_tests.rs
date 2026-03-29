use super::*;

#[test]
fn delegate_status_serialization() {
    let status = DelegateStatus::Success;
    let json = serde_json::to_string(&status).unwrap();
    let back: DelegateStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(back, DelegateStatus::Success);
}

#[test]
fn delegate_status_all_variants() {
    for variant in [
        DelegateStatus::Success,
        DelegateStatus::Failed,
        DelegateStatus::TimedOut,
        DelegateStatus::Cancelled,
    ] {
        let json = serde_json::to_string(&variant).unwrap();
        let back: DelegateStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, variant);
    }
}

#[test]
fn delegate_result_serialization() {
    let result = DelegateResult {
        status: DelegateStatus::Success,
        output: "all tests pass".to_string(),
        tokens_used: 4200,
        duration: std::time::Duration::from_secs(120),
        peer_name: "m1pro".to_string(),
        worktree_path: Some("/tmp/wt".to_string()),
    };
    let json = serde_json::to_string(&result).unwrap();
    let back: DelegateResult = serde_json::from_str(&json).unwrap();
    assert_eq!(back.tokens_used, 4200);
    assert_eq!(back.peer_name, "m1pro");
    assert_eq!(back.worktree_path.as_deref(), Some("/tmp/wt"));
}

#[test]
fn delegate_result_no_worktree() {
    let result = DelegateResult {
        status: DelegateStatus::Failed,
        output: "error".to_string(),
        tokens_used: 0,
        duration: std::time::Duration::from_secs(5),
        peer_name: "worker".to_string(),
        worktree_path: None,
    };
    let json = serde_json::to_string(&result).unwrap();
    let back: DelegateResult = serde_json::from_str(&json).unwrap();
    assert!(back.worktree_path.is_none());
    assert_eq!(back.status, DelegateStatus::Failed);
}

#[test]
fn delegate_error_display() {
    let err = DelegateError::PeerNotFound("ghost".into());
    assert!(err.to_string().contains("ghost"));

    let err = DelegateError::PeerInactive("worker".into(), "offline".into());
    assert!(err.to_string().contains("worker"));
    assert!(err.to_string().contains("offline"));

    let err = DelegateError::SshConnect("timeout".into());
    assert!(err.to_string().contains("SSH"));

    let err = DelegateError::DaemonUnhealthy("node".into(), "failed".into());
    assert!(err.to_string().contains("healthy"));

    let err = DelegateError::Timeout(std::time::Duration::from_secs(1800));
    assert!(err.to_string().contains("1800"));
}

#[test]
fn delegate_error_from_peers_error() {
    use crate::mesh::peers::PeersError;
    let peers_err = PeersError::NotFound("test".into());
    let delegate_err: DelegateError = peers_err.into();
    assert!(matches!(delegate_err, DelegateError::PeersConfig(_)));
}
