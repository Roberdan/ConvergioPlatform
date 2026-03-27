use super::types::{AclRule, Permission, ResourceType, SecurityError};
use std::collections::HashMap;
use std::sync::RwLock;

/// Per-agent Access Control List system. Deny by default.
/// Rules loaded from config/security/agents/{name}.yaml.
pub struct AclManager {
    rules: RwLock<HashMap<String, Vec<AclRule>>>,
}

impl AclManager {
    pub fn new() -> Self {
        Self {
            rules: RwLock::new(HashMap::new()),
        }
    }

    /// Load ACL rules for an agent.
    pub fn set_rules(&self, agent_id: &str, rules: Vec<AclRule>) {
        if let Ok(mut map) = self.rules.write() {
            map.insert(agent_id.to_string(), rules);
        }
    }

    /// Check if an agent has permission for a resource.
    pub fn check(
        &self,
        agent_id: &str,
        resource_type: ResourceType,
        resource: &str,
        permission: Permission,
    ) -> Result<(), SecurityError> {
        let rules = self.rules.read().map_err(|e| {
            SecurityError::ConfigError(format!("lock: {e}"))
        })?;

        let agent_rules = rules.get(agent_id).ok_or_else(|| {
            SecurityError::AccessDenied(format!("no ACL for agent {agent_id}"))
        })?;

        let matched = agent_rules.iter().any(|rule| {
            rule.resource_type == resource_type
                && matches_pattern(&rule.pattern, resource)
                && rule.permissions.contains(&permission)
        });

        if matched {
            Ok(())
        } else {
            Err(SecurityError::AccessDenied(format!(
                "{agent_id} denied {permission:?} on {resource_type:?}:{resource}"
            )))
        }
    }

    /// List rules for an agent.
    pub fn get_rules(&self, agent_id: &str) -> Vec<AclRule> {
        self.rules
            .read()
            .ok()
            .and_then(|r| r.get(agent_id).cloned())
            .unwrap_or_default()
    }
}

impl Default for AclManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Glob-like pattern matching for resource paths.
fn matches_pattern(pattern: &str, resource: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix("/*") {
        return resource.starts_with(prefix);
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        return resource.ends_with(&format!(".{suffix}"));
    }
    pattern == resource
}
