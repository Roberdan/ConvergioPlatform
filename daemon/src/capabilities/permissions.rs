use super::ring::Ring;
use super::types::CapabilityError;
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

/// Per-agent permission set for capability access control.
/// Deny by default — agents must have explicit allowed_rings or allowed_tools.
pub struct PermissionManager {
    agents: RwLock<HashMap<String, AgentPermissions>>,
}

/// Permission configuration for a single agent.
#[derive(Debug, Clone)]
pub struct AgentPermissions {
    /// Maximum ring level the agent can access (0=Core, 3=Sandboxed).
    pub max_ring: Ring,
    /// Explicitly allowed tool names (overrides ring restriction).
    pub allowed_tools: HashSet<String>,
    /// Explicitly denied tool names (overrides allowed).
    pub denied_tools: HashSet<String>,
}

impl Default for AgentPermissions {
    fn default() -> Self {
        Self {
            max_ring: Ring::Sandboxed,
            allowed_tools: HashSet::new(),
            denied_tools: HashSet::new(),
        }
    }
}

impl PermissionManager {
    pub fn new() -> Self {
        Self {
            agents: RwLock::new(HashMap::new()),
        }
    }

    /// Set permissions for an agent.
    pub fn set(&self, agent_id: &str, perms: AgentPermissions) {
        if let Ok(mut agents) = self.agents.write() {
            agents.insert(agent_id.to_string(), perms);
        }
    }

    /// Get permissions for an agent (default: Sandboxed, no tools).
    pub fn get(&self, agent_id: &str) -> AgentPermissions {
        self.agents
            .read()
            .ok()
            .and_then(|a| a.get(agent_id).cloned())
            .unwrap_or_default()
    }

    /// Check if an agent can invoke a specific tool at a given ring.
    pub fn check(
        &self,
        agent_id: &str,
        tool_name: &str,
        tool_ring: Ring,
    ) -> Result<(), CapabilityError> {
        let perms = self.get(agent_id);

        // Explicit deny always wins.
        if perms.denied_tools.contains(tool_name) {
            return Err(CapabilityError::PermissionDenied(format!(
                "{agent_id} denied for {tool_name}"
            )));
        }

        // Explicit allow bypasses ring check.
        if perms.allowed_tools.contains(tool_name) {
            return Ok(());
        }

        // Ring-based access.
        if perms.max_ring.can_access(tool_ring) {
            Ok(())
        } else {
            Err(CapabilityError::RingViolation {
                agent: perms.max_ring.as_u8(),
                capability: tool_ring.as_u8(),
            })
        }
    }

    /// Grant access to a specific tool.
    pub fn grant(&self, agent_id: &str, tool_name: &str) {
        if let Ok(mut agents) = self.agents.write() {
            let perms = agents.entry(agent_id.to_string()).or_default();
            perms.allowed_tools.insert(tool_name.to_string());
            perms.denied_tools.remove(tool_name);
        }
    }

    /// Revoke access to a specific tool.
    pub fn revoke(&self, agent_id: &str, tool_name: &str) {
        if let Ok(mut agents) = self.agents.write() {
            let perms = agents.entry(agent_id.to_string()).or_default();
            perms.denied_tools.insert(tool_name.to_string());
            perms.allowed_tools.remove(tool_name);
        }
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_sandboxed() {
        let pm = PermissionManager::new();
        let perms = pm.get("unknown-agent");
        assert_eq!(perms.max_ring, Ring::Sandboxed);
    }

    #[test]
    fn core_agent_can_access_core_tools() {
        let pm = PermissionManager::new();
        pm.set("admin", AgentPermissions {
            max_ring: Ring::Core,
            ..Default::default()
        });
        pm.check("admin", "any-tool", Ring::Core).unwrap();
    }

    #[test]
    fn sandboxed_cannot_access_core() {
        let pm = PermissionManager::new();
        let err = pm.check("random", "core-tool", Ring::Core).unwrap_err();
        assert!(matches!(err, CapabilityError::RingViolation { .. }));
    }

    #[test]
    fn explicit_allow_bypasses_ring() {
        let pm = PermissionManager::new();
        pm.grant("limited-agent", "special-tool");
        pm.check("limited-agent", "special-tool", Ring::Core).unwrap();
    }

    #[test]
    fn explicit_deny_overrides_allow() {
        let pm = PermissionManager::new();
        pm.set("agent-x", AgentPermissions {
            max_ring: Ring::Core,
            ..Default::default()
        });
        pm.revoke("agent-x", "dangerous-tool");
        let err = pm.check("agent-x", "dangerous-tool", Ring::Core).unwrap_err();
        assert!(matches!(err, CapabilityError::PermissionDenied(_)));
    }

    #[test]
    fn grant_then_revoke() {
        let pm = PermissionManager::new();
        pm.grant("agent-y", "tool-a");
        pm.check("agent-y", "tool-a", Ring::Core).unwrap();
        pm.revoke("agent-y", "tool-a");
        assert!(pm.check("agent-y", "tool-a", Ring::Core).is_err());
    }
}
