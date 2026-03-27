use super::types::{Capability, CapabilityError, ToolSchema};
use serde_json::json;

#[test]
fn tool_schema_serializes_roundtrip() {
    let schema = ToolSchema {
        name: "create_customer".to_string(),
        description: "Create a new customer".to_string(),
        input_schema: json!({"type": "object", "properties": {"name": {"type": "string"}}}),
        output_schema: json!({"type": "object", "properties": {"id": {"type": "string"}}}),
    };
    let json_str = serde_json::to_string(&schema).expect("serialize");
    let restored: ToolSchema = serde_json::from_str(&json_str).expect("deserialize");
    assert_eq!(restored.name, "create_customer");
}

#[test]
fn capability_ring_level() {
    let cap = Capability {
        name: "test-tool".to_string(),
        description: "A test tool".to_string(),
        ring: 2,
        mcp_server: None,
        input_schema: json!({}),
        permissions_required: vec![],
        enabled: true,
    };
    assert_eq!(cap.ring_level(), super::Ring::Community);
}

#[test]
fn capability_with_mcp_server() {
    let cap = Capability {
        name: "stripe-tool".to_string(),
        description: "Stripe API".to_string(),
        ring: 2,
        mcp_server: Some("stdio://stripe-mcp-server".to_string()),
        input_schema: json!({"type": "object"}),
        permissions_required: vec!["stripe:read".to_string()],
        enabled: true,
    };
    assert!(cap.mcp_server.is_some());
    assert_eq!(cap.permissions_required.len(), 1);
}

#[test]
fn capability_error_variants() {
    let e1 = CapabilityError::NotFound("x".to_string());
    let e2 = CapabilityError::PermissionDenied("no".to_string());
    let e3 = CapabilityError::RingViolation { agent: 2, capability: 0 };
    assert!(format!("{e1}").contains("not found"));
    assert!(format!("{e2}").contains("permission"));
    assert!(format!("{e3}").contains("ring violation"));
}
