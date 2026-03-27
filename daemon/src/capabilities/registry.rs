use super::ring::Ring;
use super::types::{Capability, CapabilityError};
use std::collections::HashMap;
use std::sync::RwLock;

/// In-memory registry of capabilities with ring-based access control.
pub struct CapabilityRegistry {
    capabilities: RwLock<HashMap<String, Capability>>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            capabilities: RwLock::new(HashMap::new()),
        }
    }

    /// Register a capability. Overwrites if name already exists.
    pub fn register(&self, cap: Capability) -> Result<(), CapabilityError> {
        let name = cap.name.clone();
        let mut caps = self
            .capabilities
            .write()
            .map_err(|e| CapabilityError::InvocationFailed(format!("lock: {e}")))?;
        caps.insert(name, cap);
        Ok(())
    }

    /// Look up a capability by name.
    pub fn get(&self, name: &str) -> Result<Capability, CapabilityError> {
        let caps = self
            .capabilities
            .read()
            .map_err(|e| CapabilityError::InvocationFailed(format!("lock: {e}")))?;
        caps.get(name)
            .cloned()
            .ok_or_else(|| CapabilityError::NotFound(name.to_string()))
    }

    /// List all capabilities, optionally filtered by ring level.
    pub fn list(&self, ring_filter: Option<Ring>) -> Vec<Capability> {
        let caps = self.capabilities.read().unwrap_or_else(|e| e.into_inner());
        let mut result: Vec<Capability> = caps
            .values()
            .filter(|c| match ring_filter {
                Some(r) => c.ring == r.as_u8(),
                None => true,
            })
            .cloned()
            .collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }

    /// Remove a capability by name.
    pub fn unregister(&self, name: &str) -> Result<(), CapabilityError> {
        let mut caps = self
            .capabilities
            .write()
            .map_err(|e| CapabilityError::InvocationFailed(format!("lock: {e}")))?;
        caps.remove(name)
            .map(|_| ())
            .ok_or_else(|| CapabilityError::NotFound(name.to_string()))
    }

    /// Check if an agent at the given ring can invoke a capability.
    pub fn check_access(
        &self,
        capability_name: &str,
        agent_ring: Ring,
    ) -> Result<(), CapabilityError> {
        let cap = self.get(capability_name)?;
        let cap_ring = Ring::from_u8(cap.ring);
        if agent_ring.can_access(cap_ring) {
            Ok(())
        } else {
            Err(CapabilityError::RingViolation {
                agent: agent_ring.as_u8(),
                capability: cap.ring,
            })
        }
    }

    /// Count registered capabilities.
    pub fn count(&self) -> usize {
        self.capabilities
            .read()
            .map(|c| c.len())
            .unwrap_or(0)
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}
