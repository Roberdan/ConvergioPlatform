// Popup action key handling tests — mesh provision/heartbeat and agent stop.
// Tests verify that pressing action keys in a popup sets action_pending with
// the correct (action_label, target_name) pair extracted from the popup title.

use super::super::input::{handle_key, InteractiveState};
use super::super::views::popup::{PopupContent, PopupSection};
use crossterm::event::{KeyCode, KeyModifiers};

fn mesh_node_popup(node_name: &str) -> PopupContent {
    PopupContent {
        title: format!("Node: {node_name}"),
        sections: vec![PopupSection {
            label: "Health".to_string(),
            lines: vec!["online: yes".to_string()],
        }],
        actions: vec![
            ('p', "Provision".to_string()),
            ('h', "Heartbeat".to_string()),
        ],
    }
}

fn agent_popup(agent_name: &str) -> PopupContent {
    PopupContent {
        title: format!("Agent: {agent_name}"),
        sections: vec![PopupSection {
            label: "Activity".to_string(),
            lines: vec!["status: running".to_string()],
        }],
        actions: vec![('s', "Stop Agent".to_string())],
    }
}

// --- Mesh provision action ---

#[test]
fn provision_key_sets_action_pending_with_node_name() {
    let mut state = InteractiveState {
        popup_open: true,
        popup_content: Some(mesh_node_popup("macProM1")),
        ..Default::default()
    };
    handle_key(KeyCode::Char('p'), KeyModifiers::NONE, &mut state);
    let (action, target) = state.action_pending.expect("action_pending must be set");
    assert_eq!(action, "Provision");
    assert_eq!(target, "macProM1");
}

#[test]
fn provision_key_closes_popup() {
    let mut state = InteractiveState {
        popup_open: true,
        popup_content: Some(mesh_node_popup("worker-1")),
        ..Default::default()
    };
    handle_key(KeyCode::Char('p'), KeyModifiers::NONE, &mut state);
    assert!(!state.popup_open, "popup must close after action key");
    assert!(state.popup_content.is_none(), "popup_content must be cleared");
}

// --- Mesh heartbeat action ---

#[test]
fn heartbeat_key_sets_action_pending() {
    let mut state = InteractiveState {
        popup_open: true,
        popup_content: Some(mesh_node_popup("macProM1")),
        ..Default::default()
    };
    handle_key(KeyCode::Char('h'), KeyModifiers::NONE, &mut state);
    let (action, target) = state.action_pending.expect("action_pending must be set");
    assert_eq!(action, "Heartbeat");
    // Heartbeat does not require a specific node target, but target is still extracted from title.
    assert_eq!(target, "macProM1");
}

#[test]
fn heartbeat_key_on_different_node_extracts_correct_target() {
    let mut state = InteractiveState {
        popup_open: true,
        popup_content: Some(mesh_node_popup("m3-max-studio")),
        ..Default::default()
    };
    handle_key(KeyCode::Char('h'), KeyModifiers::NONE, &mut state);
    let (action, target) = state.action_pending.unwrap();
    assert_eq!(action, "Heartbeat");
    assert_eq!(target, "m3-max-studio");
}

// --- Agent stop action ---

#[test]
fn stop_key_sets_action_pending_with_agent_name() {
    let mut state = InteractiveState {
        popup_open: true,
        popup_content: Some(agent_popup("Thor")),
        ..Default::default()
    };
    handle_key(KeyCode::Char('s'), KeyModifiers::NONE, &mut state);
    let (action, target) = state.action_pending.expect("action_pending must be set");
    assert_eq!(action, "Stop Agent");
    assert_eq!(target, "Thor");
}

#[test]
fn stop_key_closes_agent_popup() {
    let mut state = InteractiveState {
        popup_open: true,
        popup_content: Some(agent_popup("task-executor")),
        ..Default::default()
    };
    handle_key(KeyCode::Char('s'), KeyModifiers::NONE, &mut state);
    assert!(!state.popup_open);
    assert!(state.popup_content.is_none());
}

// --- Edge cases ---

#[test]
fn popup_with_no_colon_separator_in_title_yields_empty_target() {
    // Title without ": " means split_once returns None → target defaults to empty string.
    let mut state = InteractiveState {
        popup_open: true,
        popup_content: Some(PopupContent {
            title: "Detail".to_string(),
            sections: vec![],
            actions: vec![('x', "Xray".to_string())],
        }),
        ..Default::default()
    };
    handle_key(KeyCode::Char('x'), KeyModifiers::NONE, &mut state);
    let (action, target) = state.action_pending.unwrap();
    assert_eq!(action, "Xray");
    assert_eq!(target, "", "missing ': ' separator yields empty target");
}

#[test]
fn unknown_key_in_popup_leaves_action_pending_none() {
    let mut state = InteractiveState {
        popup_open: true,
        popup_content: Some(mesh_node_popup("macProM1")),
        ..Default::default()
    };
    // 'z' is not in the popup's action list.
    handle_key(KeyCode::Char('z'), KeyModifiers::NONE, &mut state);
    assert!(state.action_pending.is_none(), "unknown key must not set action_pending");
    assert!(state.popup_open, "popup must remain open for unknown key");
}

#[test]
fn action_pending_defaults_to_none() {
    let state = InteractiveState::default();
    assert!(state.action_pending.is_none());
}
