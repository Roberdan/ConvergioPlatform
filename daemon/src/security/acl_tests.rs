use super::acl::AclManager;
use super::types::{AclRule, Permission, ResourceType};

#[test]
fn deny_by_default() {
    let acl = AclManager::new();
    let err = acl.check("agent-x", ResourceType::Filesystem, "/etc/passwd", Permission::Read);
    assert!(err.is_err());
}

#[test]
fn allow_with_matching_rule() {
    let acl = AclManager::new();
    acl.set_rules("agent-a", vec![AclRule {
        resource_type: ResourceType::Filesystem,
        pattern: "/workspace/*".to_string(),
        permissions: vec![Permission::Read, Permission::Write],
    }]);
    acl.check("agent-a", ResourceType::Filesystem, "/workspace/file.rs", Permission::Read).unwrap();
}

#[test]
fn deny_wrong_permission() {
    let acl = AclManager::new();
    acl.set_rules("agent-b", vec![AclRule {
        resource_type: ResourceType::Filesystem,
        pattern: "/data/*".to_string(),
        permissions: vec![Permission::Read],
    }]);
    let err = acl.check("agent-b", ResourceType::Filesystem, "/data/secrets", Permission::Write);
    assert!(err.is_err());
}

#[test]
fn wildcard_matches_all() {
    let acl = AclManager::new();
    acl.set_rules("admin", vec![AclRule {
        resource_type: ResourceType::Api,
        pattern: "*".to_string(),
        permissions: vec![Permission::Read, Permission::Write, Permission::Execute],
    }]);
    acl.check("admin", ResourceType::Api, "/api/plans", Permission::Write).unwrap();
}

#[test]
fn network_rule() {
    let acl = AclManager::new();
    acl.set_rules("agent-net", vec![AclRule {
        resource_type: ResourceType::Network,
        pattern: "api.stripe.com:443".to_string(),
        permissions: vec![Permission::Read, Permission::Write],
    }]);
    acl.check("agent-net", ResourceType::Network, "api.stripe.com:443", Permission::Read).unwrap();
    assert!(acl.check("agent-net", ResourceType::Network, "evil.com:443", Permission::Read).is_err());
}

#[test]
fn get_rules_empty_for_unknown() {
    let acl = AclManager::new();
    assert!(acl.get_rules("ghost").is_empty());
}

#[test]
fn extension_pattern() {
    let acl = AclManager::new();
    acl.set_rules("agent-ext", vec![AclRule {
        resource_type: ResourceType::Filesystem,
        pattern: "*.rs".to_string(),
        permissions: vec![Permission::Read],
    }]);
    acl.check("agent-ext", ResourceType::Filesystem, "main.rs", Permission::Read).unwrap();
    assert!(acl.check("agent-ext", ResourceType::Filesystem, "main.py", Permission::Read).is_err());
}
