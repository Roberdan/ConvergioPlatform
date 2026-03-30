pub mod acl;
pub mod audit_chain;
pub mod budget;
pub mod egress;
pub mod guard;
pub mod jwt;
pub mod keychain;
pub mod kill_switch;
pub mod rbac;
pub mod sandbox;
pub mod types;

pub use guard::SecurityGuard;
pub use jwt::{AgentClaims, AgentRole};
pub use rbac::role_can_access;
pub use types::{AclRule, AuditEntry, SecurityError};

#[cfg(test)]
#[path = "acl_tests.rs"]
mod acl_tests;

#[cfg(test)]
#[path = "audit_chain_tests.rs"]
mod audit_chain_tests;

#[cfg(test)]
#[path = "guard_tests.rs"]
mod guard_tests;
