// Tests for T2-04 project switcher: Ctrl+P overlay, tab bar name, persistence.

use crossterm::event::{KeyCode, KeyModifiers};

use super::super::data::{ProjectInfo, TuiData};
use super::super::input::{handle_key, InteractiveState};

fn _make_projects() -> Vec<ProjectInfo> {
    vec![
        ProjectInfo {
            id: 1,
            name: "ConvergioPlatform".to_string(),
            path: "/Users/dev/ConvergioPlatform".to_string(),
        },
        ProjectInfo {
            id: 2,
            name: "MaranelloDesign".to_string(),
            path: "/Users/dev/MaranelloDesign".to_string(),
        },
    ]
}

// --- Ctrl+P toggle ---

#[test]
fn ctrl_p_opens_project_switcher() {
    let mut state = InteractiveState::default();
    assert!(!state.show_project_switcher, "switcher should start closed");
    handle_key(KeyCode::Char('p'), KeyModifiers::CONTROL, &mut state);
    assert!(state.show_project_switcher, "Ctrl+P must open project switcher");
}

#[test]
fn ctrl_p_closes_open_switcher() {
    let mut state = InteractiveState {
        show_project_switcher: true,
        ..Default::default()
    };
    handle_key(KeyCode::Char('p'), KeyModifiers::CONTROL, &mut state);
    assert!(!state.show_project_switcher, "Ctrl+P must toggle switcher closed");
}

// --- Navigation in switcher ---

#[test]
fn down_key_advances_switcher_selection() {
    let mut state = InteractiveState {
        show_project_switcher: true,
        project_switcher_selected: 0,
        ..Default::default()
    };
    handle_key(KeyCode::Down, KeyModifiers::NONE, &mut state);
    assert_eq!(state.project_switcher_selected, 1, "Down must advance selection");
}

#[test]
fn up_key_reverses_switcher_selection() {
    let mut state = InteractiveState {
        show_project_switcher: true,
        project_switcher_selected: 1,
        ..Default::default()
    };
    handle_key(KeyCode::Up, KeyModifiers::NONE, &mut state);
    assert_eq!(state.project_switcher_selected, 0, "Up must reverse selection");
}

#[test]
fn up_key_does_not_underflow() {
    let mut state = InteractiveState {
        show_project_switcher: true,
        project_switcher_selected: 0,
        ..Default::default()
    };
    handle_key(KeyCode::Up, KeyModifiers::NONE, &mut state);
    assert_eq!(state.project_switcher_selected, 0, "Up at 0 must stay at 0");
}

// --- Enter selects project ---

#[test]
fn enter_selects_project_from_switcher() {
    let mut state = InteractiveState {
        show_project_switcher: true,
        project_switcher_selected: 1,
        ..Default::default()
    };
    handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut state);
    // Enter while switcher open must set pending_project_switch to the selected index string
    assert!(state.pending_project_switch.is_some(), "Enter must set pending_project_switch");
    assert_eq!(state.pending_project_switch.as_deref(), Some("1"), "selected index 1 must be stored");
    // Switcher closes after selection
    assert!(!state.show_project_switcher, "switcher must close after Enter");
}

// --- Esc closes switcher ---

#[test]
fn esc_closes_project_switcher() {
    let mut state = InteractiveState {
        show_project_switcher: true,
        ..Default::default()
    };
    handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut state);
    assert!(!state.show_project_switcher, "Esc must close project switcher");
}

// --- Tab bar contains project name ---

#[test]
fn tab_bar_shows_active_project_name() {
    use super::render_to_text;
    use super::super::MainView;

    let data = TuiData {
        active_project_name: "ConvergioPlatform".to_string(),
        ..Default::default()
    };
    let text = render_to_text(&data, MainView::PlanKanban);
    assert!(
        text.contains("ConvergioPlatform"),
        "tab bar must show active project name; got:\n{text}"
    );
}

// --- Persistence helpers ---

#[test]
fn persistence_path_is_under_claude_data() {
    use super::super::persistence;
    let path = persistence::last_project_path();
    assert!(
        path.to_string_lossy().contains(".claude/data"),
        "persistence path must be under ~/.claude/data"
    );
}

#[test]
fn save_and_load_last_project_roundtrip() {
    use super::super::persistence;
    use std::fs;

    let dir = tempfile::tempdir().expect("tmpdir");
    let path = dir.path().join("last_project.txt");

    persistence::save_last_project_to(&path, "convergio");
    let loaded = persistence::load_last_project_from(&path);
    assert_eq!(loaded.as_deref(), Some("convergio"), "must round-trip project id");

    // cleanup
    let _ = fs::remove_file(&path);
}
