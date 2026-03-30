use super::*;

fn setup_test_secret() {
    let _ = JWT_SECRET.set(b"test-secret-key-32-bytes-long!!!".to_vec());
}

#[test]
fn issue_and_validate_roundtrip() {
    setup_test_secret();
    let token = issue_token(
        "planner-opus",
        AgentRole::Coordinator,
        vec!["plan:read".into(), "plan:write".into()],
        3600,
    )
    .expect("issue");
    let claims = validate_token(&token).expect("validate");
    assert_eq!(claims.sub, "planner-opus");
    assert_eq!(claims.role, AgentRole::Coordinator);
    assert_eq!(claims.cap.len(), 2);
    assert!(claims.exp > claims.iat);
}

#[test]
fn tampered_token_rejected() {
    setup_test_secret();
    let mut token = issue_token(
        "agent-x",
        AgentRole::Worker,
        vec![],
        3600,
    )
    .expect("issue");
    // Tamper with the payload
    let parts: Vec<&str> = token.splitn(3, '.').collect();
    let tampered = format!("{}.{}.{}", parts[0], "dGFtcGVyZWQ", parts[2]);
    token = tampered;
    assert!(matches!(
        validate_token(&token),
        Err(JwtError::InvalidSignature)
    ));
}

#[test]
fn expired_token_rejected() {
    setup_test_secret();
    // Issue a token that expired 1 second ago
    let token = issue_token("old-agent", AgentRole::Worker, vec![], 0)
        .expect("issue");
    // Token with ttl=0 means exp == iat, which is <= now
    std::thread::sleep(std::time::Duration::from_millis(1100));
    assert!(matches!(
        validate_token(&token),
        Err(JwtError::Expired)
    ));
}

#[test]
fn malformed_token_rejected() {
    setup_test_secret();
    assert!(matches!(
        validate_token("not.a.valid.token"),
        Err(JwtError::InvalidFormat)
    ));
    assert!(matches!(
        validate_token("onlyonepart"),
        Err(JwtError::InvalidFormat)
    ));
    assert!(matches!(
        validate_token("two.parts"),
        Err(JwtError::InvalidFormat)
    ));
}

#[test]
fn role_serialization_roundtrip() {
    let roles = vec![
        AgentRole::Coordinator,
        AgentRole::Executor,
        AgentRole::Kernel,
        AgentRole::Worker,
        AgentRole::Dashboard,
    ];
    for role in roles {
        let json = serde_json::to_string(&role).unwrap();
        let back: AgentRole = serde_json::from_str(&json).unwrap();
        assert_eq!(back, role);
    }
}

#[test]
fn display_formats_match_serde() {
    assert_eq!(AgentRole::Coordinator.to_string(), "coordinator");
    assert_eq!(AgentRole::Executor.to_string(), "executor");
    assert_eq!(AgentRole::Kernel.to_string(), "kernel");
    assert_eq!(AgentRole::Worker.to_string(), "worker");
    assert_eq!(AgentRole::Dashboard.to_string(), "dashboard");
}
