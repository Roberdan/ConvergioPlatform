//! W5: Distributed Intelligence — gossip, capabilities, scheduling, budget tracking.

mod hub;
mod types;

pub use hub::IntelligenceHub;
pub use types::{
    GossipMember, MemberState, NodeBudget, NodeCapability, ScheduledTask, TaskQueueStatus,
};

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_member(id: &str, state: MemberState) -> GossipMember {
        GossipMember {
            node_id: id.into(),
            addr: format!("{id}:9420"),
            incarnation: 1,
            state,
            last_seen: crate::mesh::daemon::now_ts(),
            capabilities: vec!["claude".into()],
            version: "11.5.0".into(),
        }
    }

    #[tokio::test]
    async fn gossip_registers_and_prunes_members() {
        let hub = IntelligenceHub::new();
        hub.update_member(test_member("n1", MemberState::Alive))
            .await;
        hub.update_member(test_member("n2", MemberState::Alive))
            .await;
        assert_eq!(hub.members.read().await.len(), 2);
    }

    #[tokio::test]
    async fn scheduler_picks_cheapest_available_node() {
        let hub = IntelligenceHub::new();
        hub.update_member(test_member("expensive", MemberState::Alive))
            .await;
        hub.update_member(test_member("cheap", MemberState::Alive))
            .await;

        hub.register_capabilities(
            "expensive",
            vec![NodeCapability {
                model_name: "gpt-5.3-codex".into(),
                provider: "openai".into(),
                max_tokens: 128000,
                cost_per_1k_tokens: 0.15,
                available: true,
            }],
        )
        .await;
        hub.register_capabilities(
            "cheap",
            vec![NodeCapability {
                model_name: "gpt-5.3-codex".into(),
                provider: "openai".into(),
                max_tokens: 128000,
                cost_per_1k_tokens: 0.03,
                available: true,
            }],
        )
        .await;

        let task = ScheduledTask {
            task_id: "T1".into(),
            plan_id: 599,
            model_hint: "gpt-5.3-codex".into(),
            effort: 2,
            assigned_node: None,
            status: TaskQueueStatus::Queued,
            created_at: 0,
        };
        let best = hub.schedule_task(&task).await;
        assert_eq!(best.as_deref(), Some("cheap"));
    }

    #[tokio::test]
    async fn scheduler_skips_over_budget_nodes() {
        let hub = IntelligenceHub::new();
        hub.update_member(test_member("rich", MemberState::Alive))
            .await;
        hub.update_member(test_member("broke", MemberState::Alive))
            .await;

        for node in &["rich", "broke"] {
            hub.register_capabilities(
                node,
                vec![NodeCapability {
                    model_name: "claude".into(),
                    provider: "anthropic".into(),
                    max_tokens: 200000,
                    cost_per_1k_tokens: 0.01,
                    available: true,
                }],
            )
            .await;
        }

        hub.budgets.write().await.insert(
            "broke".into(),
            NodeBudget {
                node_id: "broke".into(),
                daily_limit_usd: 10.0,
                spent_today_usd: 10.0,
                monthly_limit_usd: 300.0,
                spent_month_usd: 10.0,
                last_reset: "2026-03-10".into(),
            },
        );
        hub.budgets.write().await.insert(
            "rich".into(),
            NodeBudget {
                node_id: "rich".into(),
                daily_limit_usd: 100.0,
                spent_today_usd: 5.0,
                monthly_limit_usd: 3000.0,
                spent_month_usd: 50.0,
                last_reset: "2026-03-10".into(),
            },
        );

        let task = ScheduledTask {
            task_id: "T2".into(),
            plan_id: 599,
            model_hint: "claude".into(),
            effort: 3,
            assigned_node: None,
            status: TaskQueueStatus::Queued,
            created_at: 0,
        };
        let best = hub.schedule_task(&task).await;
        assert_eq!(best.as_deref(), Some("rich"));
    }
}
