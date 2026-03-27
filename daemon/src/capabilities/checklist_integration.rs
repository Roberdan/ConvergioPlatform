use super::ring::Ring;
use super::types::{Capability, CapabilityError};

/// Security review requirements for capability ring promotion.
/// When a capability is registered or promoted to a higher ring,
/// a security checklist must be passed.
pub struct SecurityGate;

/// Result of a security gate check.
#[derive(Debug)]
pub struct GateResult {
    pub passed: bool,
    pub checks: Vec<GateCheck>,
}

#[derive(Debug)]
pub struct GateCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

impl SecurityGate {
    /// Validate a capability registration meets security requirements.
    pub fn validate_registration(cap: &Capability) -> GateResult {
        let mut checks = Vec::new();

        // G-01: Name must be non-empty and lowercase-kebab
        checks.push(GateCheck {
            name: "valid_name".to_string(),
            passed: is_valid_name(&cap.name),
            detail: format!("name '{}' format check", cap.name),
        });

        // G-02: Description must be non-empty
        checks.push(GateCheck {
            name: "has_description".to_string(),
            passed: !cap.description.is_empty(),
            detail: "description must be non-empty".to_string(),
        });

        // G-03: Ring 0/1 require explicit permissions
        let ring = Ring::from_u8(cap.ring);
        let needs_perms = matches!(ring, Ring::Core | Ring::Trusted);
        checks.push(GateCheck {
            name: "privileged_ring_permissions".to_string(),
            passed: !needs_perms || !cap.permissions_required.is_empty(),
            detail: format!("ring {} requires permissions_required", cap.ring),
        });

        // G-04: MCP tools must have input_schema
        let has_schema = cap.input_schema.as_object().map(|o| !o.is_empty()).unwrap_or(false);
        checks.push(GateCheck {
            name: "mcp_has_schema".to_string(),
            passed: cap.mcp_server.is_none() || has_schema,
            detail: "MCP tools must define input_schema".to_string(),
        });

        // G-05: No wildcard permissions
        let has_wildcard = cap.permissions_required.iter().any(|p| p.contains('*'));
        checks.push(GateCheck {
            name: "no_wildcard_permissions".to_string(),
            passed: !has_wildcard,
            detail: "wildcard permissions not allowed".to_string(),
        });

        let passed = checks.iter().all(|c| c.passed);
        GateResult { passed, checks }
    }

    /// Validate a ring promotion (e.g., Ring 2 → Ring 1).
    pub fn validate_promotion(cap: &Capability, target_ring: Ring) -> GateResult {
        let mut base = Self::validate_registration(cap);

        // P-01: Cannot promote above current ring
        let current = Ring::from_u8(cap.ring);
        base.checks.push(GateCheck {
            name: "valid_promotion_direction".to_string(),
            passed: target_ring < current,
            detail: format!("promote from ring {} to ring {}", cap.ring, target_ring.as_u8()),
        });

        // P-02: Promotion to Ring 0 requires security audit
        base.checks.push(GateCheck {
            name: "core_promotion_audit".to_string(),
            passed: target_ring != Ring::Core || cap.permissions_required.len() >= 2,
            detail: "Ring 0 promotion requires >=2 explicit permissions".to_string(),
        });

        base.passed = base.checks.iter().all(|c| c.passed);
        base
    }
}

fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cap(name: &str, ring: u8, perms: &[&str]) -> Capability {
        Capability {
            name: name.to_string(),
            description: "Test capability".to_string(),
            ring,
            mcp_server: None,
            input_schema: json!({"type": "object"}),
            permissions_required: perms.iter().map(|s| s.to_string()).collect(),
            enabled: true,
        }
    }

    #[test]
    fn valid_community_tool_passes() {
        let c = cap("read-logs", 2, &[]);
        let result = SecurityGate::validate_registration(&c);
        assert!(result.passed);
    }

    #[test]
    fn core_tool_without_permissions_fails() {
        let c = cap("admin-reset", 0, &[]);
        let result = SecurityGate::validate_registration(&c);
        assert!(!result.passed);
    }

    #[test]
    fn core_tool_with_permissions_passes() {
        let c = cap("admin-reset", 0, &["admin:write", "system:reset"]);
        let result = SecurityGate::validate_registration(&c);
        assert!(result.passed);
    }

    #[test]
    fn wildcard_permission_fails() {
        let c = cap("broad-tool", 2, &["*"]);
        let result = SecurityGate::validate_registration(&c);
        assert!(!result.passed);
    }

    #[test]
    fn empty_name_fails() {
        let c = cap("", 2, &[]);
        let result = SecurityGate::validate_registration(&c);
        assert!(!result.passed);
    }

    #[test]
    fn mcp_tool_without_schema_fails() {
        let mut c = cap("stripe-tool", 2, &[]);
        c.mcp_server = Some("stdio://stripe".to_string());
        c.input_schema = json!({});
        let result = SecurityGate::validate_registration(&c);
        assert!(!result.passed);
    }

    #[test]
    fn promotion_validates_direction() {
        let c = cap("tool-x", 2, &[]);
        let result = SecurityGate::validate_promotion(&c, Ring::Trusted);
        assert!(result.checks.iter().any(|g| g.name == "valid_promotion_direction" && g.passed));
    }
}
