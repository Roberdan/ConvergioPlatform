// Tests for tui/widgets/shared.rs — mesh and pipeline rendering.
use super::*;
use crate::tui::{
    data::{DelegationInfo, MeshNode, TaskPipelineItem},
    TuiData,
};

fn sample_pipeline() -> Vec<TaskPipelineItem> {
    vec![TaskPipelineItem {
        task_id: "T1-01".to_string(),
        title: "Fix column alignment".to_string(),
        status: "in_progress".to_string(),
        agent: "executor".to_string(),
    }]
}

fn sample_mesh() -> Vec<MeshNode> {
    vec![MeshNode {
        name: "macProM1".to_string(),
        online: true,
        role: "coordinator".to_string(),
        cpu_percent: 42.0,
    }]
}

// --- Pipeline column-width tests ---

#[test]
fn pipeline_header_uses_12_char_agent_column() {
    // Header must use {:<12} for agent, not {:<10}
    let data = TuiData {
        pipeline: sample_pipeline(),
        ..TuiData::default()
    };
    let p = task_pipeline(&data, 0);
    let debug = format!("{p:?}");
    assert!(
        debug.contains("Agent        "),
        "Pipeline header must pad Agent to 12 chars: {debug}"
    );
}

#[test]
fn pipeline_row_agent_padded_to_12() {
    let data = TuiData {
        pipeline: sample_pipeline(),
        ..TuiData::default()
    };
    let p = task_pipeline(&data, 0);
    let debug = format!("{p:?}");
    // "executor    " — 8 chars + 4 spaces = 12
    assert!(
        debug.contains("executor    "),
        "Pipeline row must pad agent to 12 chars: {debug}"
    );
}

// --- Mesh column-width tests ---

#[test]
fn mesh_header_uses_correct_labels() {
    let data = TuiData {
        mesh_nodes: sample_mesh(),
        ..TuiData::default()
    };
    let p = mesh_status(&data, 0);
    let debug = format!("{p:?}");
    assert!(debug.contains("Node"), "Mesh header must contain Node: {debug}");
    assert!(debug.contains("Role"), "Mesh header must contain Role: {debug}");
    assert!(debug.contains("CPU"), "Mesh header must contain CPU: {debug}");
    assert!(debug.contains("Load"), "Mesh header must contain Load: {debug}");
}

#[test]
fn mesh_row_name_padded_to_16() {
    let data = TuiData {
        mesh_nodes: sample_mesh(),
        ..TuiData::default()
    };
    let p = mesh_status(&data, 0);
    let debug = format!("{p:?}");
    // "macProM1        " — 8 chars padded to 16
    assert!(
        debug.contains("macProM1        "),
        "Mesh row must pad name to 16 chars: {debug}"
    );
}

#[test]
fn mesh_row_role_padded_to_12() {
    let data = TuiData {
        mesh_nodes: sample_mesh(),
        ..TuiData::default()
    };
    let p = mesh_status(&data, 0);
    let debug = format!("{p:?}");
    // "coordinator " — 11 chars + 1 space = 12
    assert!(
        debug.contains("coordinator "),
        "Mesh row must pad role to 12 chars: {debug}"
    );
}

// --- Delegation rendering tests ---

#[test]
fn mesh_delegation_shows_peer_name_and_progress() {
    // A delegated plan must render peer name, plan name, and progress.
    let data = TuiData {
        mesh_nodes: vec![MeshNode {
            name: "macMiniM2".to_string(),
            online: true,
            role: "worker".to_string(),
            cpu_percent: 55.0,
        }],
        delegations: vec![DelegationInfo {
            peer_name: "macMiniM2".to_string(),
            plan_name: "Plan H0".to_string(),
            plan_id: 719,
            tasks_done: 3,
            tasks_total: 8,
            agent_name: "task-executor".to_string(),
            last_heartbeat: "2s ago".to_string(),
        }],
        ..TuiData::default()
    };
    let p = mesh_status(&data, 0);
    let debug = format!("{p:?}");
    assert!(debug.contains("Plan H0"), "Delegation must show plan name: {debug}");
    assert!(debug.contains("3/8"), "Delegation must show tasks_done/tasks_total: {debug}");
    assert!(debug.contains("task-executor"), "Delegation must show agent name: {debug}");
    assert!(debug.contains("2s ago"), "Delegation must show last heartbeat: {debug}");
}

#[test]
fn mesh_node_without_delegation_shows_basic_info_only() {
    // A node with no delegations must not render any arrow lines.
    let data = TuiData {
        mesh_nodes: vec![MeshNode {
            name: "macProM1".to_string(),
            online: true,
            role: "coordinator".to_string(),
            cpu_percent: 10.0,
        }],
        delegations: vec![],
        ..TuiData::default()
    };
    let p = mesh_status(&data, 0);
    let debug = format!("{p:?}");
    assert!(
        !debug.contains("\u{2192}"),
        "Node with no delegation must not show arrow lines: {debug}"
    );
}

#[test]
fn mesh_delegation_progress_format_percent() {
    // 3/8 = 37% must be rendered in the delegation row.
    let data = TuiData {
        mesh_nodes: vec![MeshNode {
            name: "macMiniM2".to_string(),
            online: true,
            role: "worker".to_string(),
            cpu_percent: 55.0,
        }],
        delegations: vec![DelegationInfo {
            peer_name: "macMiniM2".to_string(),
            plan_name: "Plan H0".to_string(),
            plan_id: 719,
            tasks_done: 3,
            tasks_total: 8,
            agent_name: "task-executor".to_string(),
            last_heartbeat: "5s ago".to_string(),
        }],
        ..TuiData::default()
    };
    let p = mesh_status(&data, 0);
    let debug = format!("{p:?}");
    assert!(debug.contains("37%"), "Delegation must show percentage progress: {debug}");
}
