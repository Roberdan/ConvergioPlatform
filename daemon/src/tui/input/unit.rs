use super::*;

fn make_popup() -> PopupContent {
    PopupContent {
        title: "Test".to_string(),
        sections: vec![],
        actions: vec![('p', "Provision".to_string()), ('s', "Stop".to_string())],
    }
}

#[test]
fn esc_in_command_mode_takes_priority() {
    let mut s = InteractiveState {
        command_mode: true,
        command_input: "abc".into(),
        popup_open: true,
        popup_content: Some(make_popup()),
        ..Default::default()
    };
    handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut s);
    assert!(!s.command_mode);
    assert!(s.popup_open, "popup should survive if command mode exits first");
}

#[test]
fn esc_closes_popup() {
    let mut s = InteractiveState {
        popup_open: true,
        popup_content: Some(make_popup()),
        ..Default::default()
    };
    handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut s);
    assert!(!s.popup_open);
    assert!(s.popup_content.is_none());
}

#[test]
fn popup_action_key_sets_pending_and_closes() {
    let mut s = InteractiveState {
        popup_open: true,
        popup_content: Some(make_popup()),
        ..Default::default()
    };
    handle_key(KeyCode::Char('p'), KeyModifiers::NONE, &mut s);
    assert!(!s.popup_open);
    assert!(s.popup_content.is_none());
    let (action, _) = s.action_pending.expect("action_pending should be set");
    assert_eq!(action, "Provision");
}

#[test]
fn popup_unknown_key_is_swallowed() {
    let mut s = InteractiveState {
        popup_open: true,
        popup_content: Some(make_popup()),
        ..Default::default()
    };
    let quit = handle_key(KeyCode::Char('q'), KeyModifiers::NONE, &mut s);
    assert!(!quit, "popup should swallow quit key");
    assert!(s.popup_open, "popup should remain open");
    assert!(s.action_pending.is_none());
}

#[test]
fn r_key_sets_force_refresh() {
    let mut s = InteractiveState::default();
    handle_key(KeyCode::Char('r'), KeyModifiers::NONE, &mut s);
    assert!(s.force_refresh);
}
