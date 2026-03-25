// Tests for TUI notification inbox: data model, unread count, key bindings, rendering.
use super::super::data::{Notification, TuiData};
use super::super::input::{self, InteractiveState};
use super::super::MainView;
use super::{render_to_text, sample_data};
use crossterm::event::{KeyCode, KeyModifiers};

fn sample_notifications() -> Vec<Notification> {
    vec![
        Notification {
            id: 1,
            title: "Build failed".to_string(),
            message: "Plan 725 W1 build error".to_string(),
            severity: "error".to_string(),
            read: false,
            created_at: "2026-03-25T12:00:00Z".to_string(),
        },
        Notification {
            id: 2,
            title: "Deploy complete".to_string(),
            message: "Plan 700 deployed to staging".to_string(),
            severity: "success".to_string(),
            read: true,
            created_at: "2026-03-25T11:00:00Z".to_string(),
        },
        Notification {
            id: 3,
            title: "Mesh node offline".to_string(),
            message: "node-b went offline".to_string(),
            severity: "warning".to_string(),
            read: false,
            created_at: "2026-03-25T10:00:00Z".to_string(),
        },
    ]
}

#[test]
fn notification_struct_fields() {
    let n = Notification {
        id: 42,
        title: "Alert".to_string(),
        message: "Something happened".to_string(),
        severity: "info".to_string(),
        read: false,
        created_at: "2026-03-25T09:00:00Z".to_string(),
    };
    assert_eq!(n.id, 42);
    assert_eq!(n.title, "Alert");
    assert!(!n.read);
}

#[test]
fn tui_data_notifications_default_empty() {
    let data = TuiData::default();
    assert!(data.notifications.is_empty());
}

#[test]
fn unread_count_calculation() {
    let notifs = sample_notifications();
    let unread: usize = notifs.iter().filter(|n| !n.read).count();
    assert_eq!(unread, 2);
}

#[test]
fn n_key_toggles_notification_inbox() {
    let mut state = InteractiveState::default();
    assert!(!state.show_notifications);
    // Press 'n' to open
    input::handle_key(KeyCode::Char('n'), KeyModifiers::NONE, &mut state);
    assert!(state.show_notifications);
    // Press 'n' again to close
    input::handle_key(KeyCode::Char('n'), KeyModifiers::NONE, &mut state);
    assert!(!state.show_notifications);
}

#[test]
fn esc_closes_notification_inbox() {
    let mut state = InteractiveState::default();
    state.show_notifications = true;
    input::handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut state);
    assert!(!state.show_notifications);
}

#[test]
fn notification_inbox_swallows_quit_key() {
    let mut state = InteractiveState::default();
    state.show_notifications = true;
    // 'q' should NOT quit when notification inbox is open
    let quit = input::handle_key(KeyCode::Char('q'), KeyModifiers::NONE, &mut state);
    assert!(!quit, "notification inbox should swallow quit key");
}

#[test]
fn status_bar_shows_unread_count() {
    let mut data = sample_data();
    data.notifications = sample_notifications();
    let rendered = render_to_text(&data, MainView::PlanKanban);
    assert!(
        rendered.contains("[2 unread]"),
        "status bar must show unread notification count"
    );
}

#[test]
fn status_bar_no_badge_when_zero_unread() {
    let mut data = sample_data();
    data.notifications = vec![Notification {
        id: 1,
        title: "Old alert".to_string(),
        message: "Already read".to_string(),
        severity: "info".to_string(),
        read: true,
        created_at: "2026-03-25T09:00:00Z".to_string(),
    }];
    let rendered = render_to_text(&data, MainView::PlanKanban);
    assert!(
        !rendered.contains("unread"),
        "status bar should not show unread badge when count is zero"
    );
}
