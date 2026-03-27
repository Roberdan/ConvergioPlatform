use super::registry::CapabilityRegistry;
use super::ring::Ring;
use super::types::Capability;
use serde_json::json;

fn make_cap(name: &str, ring: u8) -> Capability {
    Capability {
        name: name.to_string(),
        description: format!("Test capability {name}"),
        ring,
        mcp_server: None,
        input_schema: json!({"type": "object"}),
        permissions_required: vec![],
        enabled: true,
    }
}

#[test]
fn register_and_get() {
    let reg = CapabilityRegistry::new();
    reg.register(make_cap("read-file", 0)).unwrap();
    let cap = reg.get("read-file").unwrap();
    assert_eq!(cap.name, "read-file");
    assert_eq!(cap.ring, 0);
}

#[test]
fn get_nonexistent_returns_not_found() {
    let reg = CapabilityRegistry::new();
    assert!(reg.get("nope").is_err());
}

#[test]
fn list_all() {
    let reg = CapabilityRegistry::new();
    reg.register(make_cap("tool-a", 0)).unwrap();
    reg.register(make_cap("tool-b", 1)).unwrap();
    reg.register(make_cap("tool-c", 2)).unwrap();
    let all = reg.list(None);
    assert_eq!(all.len(), 3);
}

#[test]
fn list_filtered_by_ring() {
    let reg = CapabilityRegistry::new();
    reg.register(make_cap("core-1", 0)).unwrap();
    reg.register(make_cap("core-2", 0)).unwrap();
    reg.register(make_cap("community-1", 2)).unwrap();
    let core_only = reg.list(Some(Ring::Core));
    assert_eq!(core_only.len(), 2);
    let community = reg.list(Some(Ring::Community));
    assert_eq!(community.len(), 1);
}

#[test]
fn unregister() {
    let reg = CapabilityRegistry::new();
    reg.register(make_cap("temp", 1)).unwrap();
    assert_eq!(reg.count(), 1);
    reg.unregister("temp").unwrap();
    assert_eq!(reg.count(), 0);
}

#[test]
fn unregister_nonexistent_returns_error() {
    let reg = CapabilityRegistry::new();
    assert!(reg.unregister("ghost").is_err());
}

#[test]
fn check_access_allowed() {
    let reg = CapabilityRegistry::new();
    reg.register(make_cap("community-tool", 2)).unwrap();
    // Core agent can access community tool
    reg.check_access("community-tool", Ring::Core).unwrap();
    // Community agent can access community tool
    reg.check_access("community-tool", Ring::Community).unwrap();
}

#[test]
fn check_access_denied() {
    let reg = CapabilityRegistry::new();
    reg.register(make_cap("core-secret", 0)).unwrap();
    // Sandboxed agent cannot access core tool
    let err = reg.check_access("core-secret", Ring::Sandboxed).unwrap_err();
    assert!(format!("{err}").contains("ring violation"));
}

#[test]
fn count_tracks_registrations() {
    let reg = CapabilityRegistry::new();
    assert_eq!(reg.count(), 0);
    reg.register(make_cap("a", 0)).unwrap();
    reg.register(make_cap("b", 1)).unwrap();
    assert_eq!(reg.count(), 2);
}

#[test]
fn register_overwrites_existing() {
    let reg = CapabilityRegistry::new();
    reg.register(make_cap("tool", 0)).unwrap();
    let mut updated = make_cap("tool", 2);
    updated.description = "Updated description".to_string();
    reg.register(updated).unwrap();
    assert_eq!(reg.count(), 1);
    let cap = reg.get("tool").unwrap();
    assert_eq!(cap.ring, 2);
    assert_eq!(cap.description, "Updated description");
}

#[test]
fn list_sorted_by_name() {
    let reg = CapabilityRegistry::new();
    reg.register(make_cap("zebra", 0)).unwrap();
    reg.register(make_cap("alpha", 0)).unwrap();
    reg.register(make_cap("middle", 0)).unwrap();
    let list = reg.list(None);
    assert_eq!(list[0].name, "alpha");
    assert_eq!(list[1].name, "middle");
    assert_eq!(list[2].name, "zebra");
}
