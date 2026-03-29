//! W5: Distributed Intelligence — gossip, capabilities, scheduling, budget tracking.

mod hub;
mod types;

pub use hub::IntelligenceHub;
pub use types::{
    GossipMember, MemberState, NodeBudget, NodeCapability, ScheduledTask, TaskQueueStatus,
};

#[cfg(test)]
#[path = "../intelligence_tests.rs"]
mod tests;
