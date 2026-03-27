use super::guard::SecurityGuard;
use super::types::{AclRule, Permission, ResourceType};

#[test]
fn guard_allows_with_acl() {
    let guard = SecurityGuard::new();
    guard.acl().set_rules("agent-g", vec![AclRule {
        resource_type: ResourceType::Api,
        pattern: "/api/plans/*".to_string(),
        permissions: vec![Permission::Read],
    }]);
    guard.check_and_record("agent-g", ResourceType::Api, "/api/plans/list", Permission::Read).unwrap();
    assert_eq!(guard.audit().len(), 1);
}

#[test]
fn guard_denies_without_acl() {
    let guard = SecurityGuard::new();
    let err = guard.check_and_record("agent-h", ResourceType::Filesystem, "/etc/shadow", Permission::Read);
    assert!(err.is_err());
    assert_eq!(guard.audit().len(), 0, "denied ops should not be recorded");
}

#[test]
fn guard_audit_chain_integrity() {
    let guard = SecurityGuard::new();
    guard.acl().set_rules("agent-i", vec![AclRule {
        resource_type: ResourceType::Api,
        pattern: "*".to_string(),
        permissions: vec![Permission::Read, Permission::Write],
    }]);
    guard.check_and_record("agent-i", ResourceType::Api, "/a", Permission::Read).unwrap();
    guard.check_and_record("agent-i", ResourceType::Api, "/b", Permission::Write).unwrap();
    assert!(guard.verify_integrity().unwrap());
}

#[test]
fn guard_multiple_agents_isolated() {
    let guard = SecurityGuard::new();
    guard.acl().set_rules("alice", vec![AclRule {
        resource_type: ResourceType::Filesystem,
        pattern: "/alice/*".to_string(),
        permissions: vec![Permission::Read],
    }]);
    guard.acl().set_rules("bob", vec![AclRule {
        resource_type: ResourceType::Filesystem,
        pattern: "/bob/*".to_string(),
        permissions: vec![Permission::Read],
    }]);
    guard.check_and_record("alice", ResourceType::Filesystem, "/alice/data", Permission::Read).unwrap();
    assert!(guard.check_and_record("alice", ResourceType::Filesystem, "/bob/data", Permission::Read).is_err());
}
