// W5: Distributed Intelligence types — gossip membership, capabilities, scheduling, budget.

use serde::{Deserialize, Serialize};

/// T5-01: SWIM-lite gossip state — membership + failure detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMember {
    pub node_id: String,
    pub addr: String,
    pub incarnation: u64,
    pub state: MemberState,
    pub last_seen: u64,
    pub capabilities: Vec<String>,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberState {
    Alive,
    Suspect,
    Dead,
}

/// T5-02: Capability entry — what models/tools a node supports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapability {
    pub model_name: String,
    pub provider: String,
    pub max_tokens: u32,
    pub cost_per_1k_tokens: f64,
    pub available: bool,
}

/// T5-04: Per-node budget tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeBudget {
    pub node_id: String,
    pub daily_limit_usd: f64,
    pub spent_today_usd: f64,
    pub monthly_limit_usd: f64,
    pub spent_month_usd: f64,
    pub last_reset: String,
}

/// T5-03: Task queue entry for distributed scheduling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub task_id: String,
    pub plan_id: i64,
    pub model_hint: String,
    pub effort: u8,
    pub assigned_node: Option<String>,
    pub status: TaskQueueStatus,
    pub created_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskQueueStatus {
    Queued,
    Assigned,
    Running,
    Done,
    Failed,
}
