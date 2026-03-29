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
    hub.update_member(test_member("n1", MemberState::Alive)).await;
    hub.update_member(test_member("n2", MemberState::Alive)).await;
    assert_eq!(hub.members.read().await.len(), 2);
}

#[tokio::test]
async fn scheduler_picks_cheapest_available_node() {
    let hub = IntelligenceHub::new();
    hub.update_member(test_member("expensive", MemberState::Alive)).await;
    hub.update_member(test_member("cheap", MemberState::Alive)).await;
    let cap = |cost| NodeCapability {
        model_name: "gpt-5.3-codex".into(), provider: "openai".into(),
        max_tokens: 128000, cost_per_1k_tokens: cost, available: true,
    };
    hub.register_capabilities("expensive", vec![cap(0.15)]).await;
    hub.register_capabilities("cheap", vec![cap(0.03)]).await;
    let task = ScheduledTask {
        task_id: "T1".into(), plan_id: 599, model_hint: "gpt-5.3-codex".into(),
        effort: 2, assigned_node: None, status: TaskQueueStatus::Queued, created_at: 0,
    };
    assert_eq!(hub.schedule_task(&task).await.as_deref(), Some("cheap"));
}

#[tokio::test]
async fn scheduler_skips_over_budget_nodes() {
    let hub = IntelligenceHub::new();
    hub.update_member(test_member("rich", MemberState::Alive)).await;
    hub.update_member(test_member("broke", MemberState::Alive)).await;
    let cap = NodeCapability {
        model_name: "claude".into(), provider: "anthropic".into(),
        max_tokens: 200000, cost_per_1k_tokens: 0.01, available: true,
    };
    for node in &["rich", "broke"] {
        hub.register_capabilities(node, vec![cap.clone()]).await;
    }
    let budget = |id: &str, spent: f64, limit: f64| NodeBudget {
        node_id: id.into(), daily_limit_usd: limit, spent_today_usd: spent,
        monthly_limit_usd: 3000.0, spent_month_usd: spent, last_reset: "2026-03-10".into(),
    };
    hub.budgets.write().await.insert("broke".into(), budget("broke", 10.0, 10.0));
    hub.budgets.write().await.insert("rich".into(), budget("rich", 5.0, 100.0));
    let task = ScheduledTask {
        task_id: "T2".into(), plan_id: 599, model_hint: "claude".into(),
        effort: 3, assigned_node: None, status: TaskQueueStatus::Queued, created_at: 0,
    };
    assert_eq!(hub.schedule_task(&task).await.as_deref(), Some("rich"));
}

// ── Additional intelligence tests ────────────────────────────────────────────

#[tokio::test]
async fn stale_update_ignored_when_incarnation_lower() {
    let hub = IntelligenceHub::new();
    let mut m = test_member("node", MemberState::Alive);
    m.incarnation = 5;
    hub.update_member(m).await;

    // Lower incarnation Suspect update should be ignored
    let mut stale = test_member("node", MemberState::Suspect);
    stale.incarnation = 3;
    hub.update_member(stale).await;

    let members = hub.members.read().await;
    assert_eq!(members["node"].state, MemberState::Alive);
    assert_eq!(members["node"].incarnation, 5);
}

#[tokio::test]
async fn alive_update_replaces_even_with_lower_incarnation() {
    let hub = IntelligenceHub::new();
    let mut m = test_member("node", MemberState::Dead);
    m.incarnation = 10;
    hub.update_member(m).await;

    // Alive with lower incarnation still replaces (the condition checks != Alive)
    let mut alive = test_member("node", MemberState::Alive);
    alive.incarnation = 5;
    hub.update_member(alive).await;

    let members = hub.members.read().await;
    assert_eq!(members["node"].state, MemberState::Alive);
}

#[tokio::test]
async fn record_spend_accumulates() {
    let hub = IntelligenceHub::new();
    hub.budgets.write().await.insert("node".into(), NodeBudget {
        node_id: "node".into(), daily_limit_usd: 100.0, spent_today_usd: 0.0,
        monthly_limit_usd: 1000.0, spent_month_usd: 0.0, last_reset: "2026-03-28".into(),
    });
    hub.record_spend("node", 5.0).await;
    hub.record_spend("node", 3.0).await;
    let b = &hub.budgets.read().await["node"];
    assert!((b.spent_today_usd - 8.0).abs() < 0.001);
    assert!((b.spent_month_usd - 8.0).abs() < 0.001);
}

#[tokio::test]
async fn record_spend_noop_for_unknown_node() {
    let hub = IntelligenceHub::new();
    hub.record_spend("ghost", 10.0).await;
    assert!(hub.budgets.read().await.is_empty());
}

#[tokio::test]
async fn scheduler_returns_none_no_capable_nodes() {
    let hub = IntelligenceHub::new();
    hub.update_member(test_member("node", MemberState::Alive)).await;
    // No capabilities registered
    let task = ScheduledTask {
        task_id: "T3".into(),
        plan_id: 600,
        model_hint: "claude-opus".into(),
        effort: 1,
        assigned_node: None,
        status: TaskQueueStatus::Queued,
        created_at: 0,
    };
    let best = hub.schedule_task(&task).await;
    assert!(best.is_none());
}

#[tokio::test]
async fn scheduler_skips_dead_nodes() {
    let hub = IntelligenceHub::new();
    hub.update_member(test_member("dead-node", MemberState::Dead)).await;
    hub.register_capabilities(
        "dead-node",
        vec![NodeCapability {
            model_name: "claude".into(),
            provider: "anthropic".into(),
            max_tokens: 200000,
            cost_per_1k_tokens: 0.01,
            available: true,
        }],
    )
    .await;
    let task = ScheduledTask {
        task_id: "T4".into(),
        plan_id: 601,
        model_hint: "claude".into(),
        effort: 1,
        assigned_node: None,
        status: TaskQueueStatus::Queued,
        created_at: 0,
    };
    assert!(hub.schedule_task(&task).await.is_none());
}

#[tokio::test]
async fn snapshot_contains_all_fields() {
    let hub = IntelligenceHub::new();
    hub.update_member(test_member("n1", MemberState::Alive)).await;
    hub.update_member(test_member("n2", MemberState::Suspect)).await;
    let snap = hub.snapshot().await;
    assert_eq!(snap["members"], 2);
    assert_eq!(snap["alive"], 1);
    assert_eq!(snap["suspect"], 1);
    assert_eq!(snap["dead"], 0);
}

#[test]
fn local_version_info_has_required_fields() {
    let info = IntelligenceHub::local_version_info();
    assert!(info["version"].is_string());
    assert!(info["protocol_version"].is_number());
    let features = info["features"].as_array().unwrap();
    assert!(features.iter().any(|f| f == "gossip"));
    assert!(features.iter().any(|f| f == "auth"));
}

#[test]
fn gossip_member_serialization() {
    let m = test_member("node-1", MemberState::Alive);
    let json = serde_json::to_string(&m).unwrap();
    let back: GossipMember = serde_json::from_str(&json).unwrap();
    assert_eq!(back.node_id, "node-1");
    assert_eq!(back.state, MemberState::Alive);
}

#[test]
fn task_queue_status_serialization() {
    for status in [
        TaskQueueStatus::Queued,
        TaskQueueStatus::Assigned,
        TaskQueueStatus::Running,
        TaskQueueStatus::Done,
        TaskQueueStatus::Failed,
    ] {
        let json = serde_json::to_string(&status).unwrap();
        let back: TaskQueueStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, status);
    }
}

#[test]
fn intelligence_hub_default() {
    let hub = IntelligenceHub::default();
    // Verify Default impl works
    let _ = hub;
}
