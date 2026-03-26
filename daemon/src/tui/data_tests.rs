// Tests for tui/data.rs — view model structs.
use super::*;

#[test]
fn tui_data_default_compiles_and_is_empty() {
    let data = TuiData::default();
    assert!(data.plans.is_empty());
    assert!(data.pipeline.is_empty());
    assert!(data.mesh_nodes.is_empty());
    assert!(data.agents.is_empty());
    assert!(data.brain_nodes.is_empty());
    assert!(data.events.is_empty());
    assert!(data.workspaces.is_empty());
    assert!(data.deliverables.is_empty());
    assert!(data.chat_messages.is_empty());
    assert!(data.chat_session_id.is_none());
    assert!(data.notifications.is_empty());
    assert!(data.project_tree.plans.is_empty());
    assert!(data.projects.is_empty());
    assert!(data.delegations.is_empty());
}

#[test]
fn chat_message_default_has_empty_fields() {
    let msg = ChatMessage::default();
    assert!(msg.role.is_empty());
    assert!(msg.content.is_empty());
    assert!(msg.timestamp.is_empty());
}

#[test]
fn chat_message_role_and_content_roundtrip() {
    let msg = ChatMessage {
        role: "user".to_string(),
        content: "hello".to_string(),
        timestamp: "2026-03-24T10:00:00Z".to_string(),
    };
    assert_eq!(msg.role, "user");
    assert_eq!(msg.content, "hello");
}

#[test]
fn kpi_data_default_zeroes() {
    let kpi = KpiData::default();
    assert_eq!(kpi.plans_active, 0);
    assert_eq!(kpi.agents_running, 0);
    assert_eq!(kpi.daily_tokens, 0);
    assert_eq!(kpi.daily_cost, 0.0);
    assert_eq!(kpi.mesh_online, 0);
}

#[test]
fn cost_data_default_compiles_and_is_empty() {
    let cost = CostData::default();
    assert!(cost.by_model.is_empty());
    assert!(cost.by_project.is_empty());
    assert!(cost.by_date.is_empty());
    assert_eq!(cost.summary.run_count, 0);
}

#[test]
fn delegation_info_default_is_empty() {
    let d = DelegationInfo::default();
    assert!(d.peer_name.is_empty());
    assert!(d.plan_name.is_empty());
    assert_eq!(d.plan_id, 0);
    assert_eq!(d.tasks_done, 0);
    assert_eq!(d.tasks_total, 0);
    assert!(d.agent_name.is_empty());
    assert!(d.last_heartbeat.is_empty());
}
