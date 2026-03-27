use super::acl::AclManager;
use super::audit_chain::AuditChain;
use super::types::{Permission, ResourceType, SecurityError};

/// Central security middleware. All agent operations pass through here:
/// ACL check → audit chain entry.
pub struct SecurityGuard {
    acl: AclManager,
    audit: AuditChain,
}

impl SecurityGuard {
    pub fn new() -> Self {
        Self {
            acl: AclManager::new(),
            audit: AuditChain::new(),
        }
    }

    /// Check and record an agent operation.
    pub fn check_and_record(
        &self,
        agent_id: &str,
        resource_type: ResourceType,
        resource: &str,
        permission: Permission,
    ) -> Result<(), SecurityError> {
        // ACL check.
        self.acl.check(agent_id, resource_type.clone(), resource, permission.clone())?;

        // Record in audit chain.
        let action = format!("{permission:?}");
        let target = format!("{resource_type:?}:{resource}");
        self.audit.record(agent_id, &action, &target, "")?;

        Ok(())
    }

    /// Direct access to ACL manager for configuration.
    pub fn acl(&self) -> &AclManager {
        &self.acl
    }

    /// Direct access to audit chain for queries.
    pub fn audit(&self) -> &AuditChain {
        &self.audit
    }

    /// Verify audit chain integrity.
    pub fn verify_integrity(&self) -> Result<bool, SecurityError> {
        self.audit.verify()
    }
}

impl Default for SecurityGuard {
    fn default() -> Self {
        Self::new()
    }
}
