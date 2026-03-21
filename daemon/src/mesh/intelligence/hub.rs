// IntelligenceHub: gossip, scheduling, budget tracking, and snapshot.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::{
    GossipMember, MemberState, NodeBudget, NodeCapability, ScheduledTask, TaskQueueStatus,
};

/// Central distributed intelligence state
pub struct IntelligenceHub {
    pub members: Arc<RwLock<HashMap<String, GossipMember>>>,
    pub capabilities: Arc<RwLock<HashMap<String, Vec<NodeCapability>>>>,
    pub budgets: Arc<RwLock<HashMap<String, NodeBudget>>>,
    pub task_queue: Arc<RwLock<Vec<ScheduledTask>>>,
}

impl IntelligenceHub {
    pub fn new() -> Self {
        Self {
            members: Arc::new(RwLock::new(HashMap::new())),
            capabilities: Arc::new(RwLock::new(HashMap::new())),
            budgets: Arc::new(RwLock::new(HashMap::new())),
            task_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// T5-01: Register or update a member via gossip
    pub async fn update_member(&self, member: GossipMember) {
        let mut members = self.members.write().await;
        let node_id = member.node_id.clone();
        if let Some(existing) = members.get(&node_id) {
            if member.incarnation <= existing.incarnation && member.state != MemberState::Alive {
                return; // Stale update
            }
        }
        members.insert(node_id, member);
    }

    /// T5-01: Mark suspect nodes as dead after timeout
    pub async fn prune_dead_members(&self, timeout_secs: u64) {
        let now = crate::mesh::daemon::now_ts();
        let mut members = self.members.write().await;
        for member in members.values_mut() {
            let age = now.saturating_sub(member.last_seen);
            match member.state {
                MemberState::Alive if age > timeout_secs / 2 => {
                    member.state = MemberState::Suspect;
                }
                MemberState::Suspect if age > timeout_secs => {
                    member.state = MemberState::Dead;
                }
                _ => {}
            }
        }
    }

    /// T5-02: Register node capabilities
    pub async fn register_capabilities(&self, node_id: &str, caps: Vec<NodeCapability>) {
        self.capabilities
            .write()
            .await
            .insert(node_id.to_string(), caps);
    }

    /// T5-03: Find best node for a task based on model hint + budget + availability
    pub async fn schedule_task(&self, task: &ScheduledTask) -> Option<String> {
        let members = self.members.read().await;
        let capabilities = self.capabilities.read().await;
        let budgets = self.budgets.read().await;

        let mut best: Option<(String, f64)> = None;

        for (node_id, member) in members.iter() {
            if member.state != MemberState::Alive {
                continue;
            }
            if let Some(caps) = capabilities.get(node_id) {
                let has_model = caps.iter().any(|c| {
                    c.available
                        && (c.model_name == task.model_hint || c.provider == task.model_hint)
                });
                if !has_model {
                    continue;
                }
                if let Some(budget) = budgets.get(node_id) {
                    if budget.spent_today_usd >= budget.daily_limit_usd {
                        continue; // Over budget
                    }
                }
                // Score: prefer lowest cost, then least loaded
                let cost = caps
                    .iter()
                    .filter(|c| c.model_name == task.model_hint)
                    .map(|c| c.cost_per_1k_tokens)
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap_or(f64::MAX);
                if best.as_ref().is_none_or(|(_, best_cost)| cost < *best_cost) {
                    best = Some((node_id.clone(), cost));
                }
            }
        }
        best.map(|(node, _)| node)
    }

    /// T5-04: Record spend against a node's budget
    pub async fn record_spend(&self, node_id: &str, amount_usd: f64) {
        let mut budgets = self.budgets.write().await;
        if let Some(budget) = budgets.get_mut(node_id) {
            budget.spent_today_usd += amount_usd;
            budget.spent_month_usd += amount_usd;
        }
    }

    /// Snapshot for API/dashboard
    pub async fn snapshot(&self) -> serde_json::Value {
        let members = self.members.read().await;
        let caps = self.capabilities.read().await;
        let budgets = self.budgets.read().await;
        let queue = self.task_queue.read().await;
        serde_json::json!({
            "members": members.len(),
            "alive": members.values().filter(|m| m.state == MemberState::Alive).count(),
            "suspect": members.values().filter(|m| m.state == MemberState::Suspect).count(),
            "dead": members.values().filter(|m| m.state == MemberState::Dead).count(),
            "capabilities": caps.len(),
            "budgets": budgets.len(),
            "queue_size": queue.len(),
            "queue_running": queue.iter().filter(|t| t.status == TaskQueueStatus::Running).count(),
        })
    }

    /// T5-05: Version info for peer negotiation
    pub fn local_version_info() -> serde_json::Value {
        serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "features": ["gossip", "capabilities", "scheduler", "budget", "anti-entropy", "auth"],
            "protocol_version": 2,
        })
    }
}

impl Default for IntelligenceHub {
    fn default() -> Self {
        Self::new()
    }
}
