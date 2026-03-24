// Command mode and key handling tests.
use super::super::app::TuiApp;
use super::super::input::{handle_key, parse_and_apply_command, InteractiveState};
use super::super::MainView;

#[test]
fn handle_command_slash_enters_command_mode() {
    let mut state = InteractiveState::default();
    assert!(!state.command_mode, "should start outside command mode");
    handle_key(
        crossterm::event::KeyCode::Char('/'),
        crossterm::event::KeyModifiers::NONE,
        &mut state,
    );
    assert!(state.command_mode, "slash must enter command mode");
    assert!(state.command_input.is_empty(), "input starts empty");
}

#[test]
fn handle_command_chars_append_in_command_mode() {
    let mut state = InteractiveState::default();
    state.command_mode = true;
    for ch in ['p', 'l', 'a', 'n'] {
        handle_key(
            crossterm::event::KeyCode::Char(ch),
            crossterm::event::KeyModifiers::NONE,
            &mut state,
        );
    }
    assert_eq!(state.command_input, "plan");
}

#[test]
fn handle_command_backspace_removes_last_char() {
    let mut state = InteractiveState {
        command_mode: true,
        command_input: "pla".into(),
        ..Default::default()
    };
    handle_key(
        crossterm::event::KeyCode::Backspace,
        crossterm::event::KeyModifiers::NONE,
        &mut state,
    );
    assert_eq!(state.command_input, "pl");
}

#[test]
fn handle_command_esc_exits_command_mode() {
    let mut state = InteractiveState {
        command_mode: true,
        command_input: "plan 708".into(),
        ..Default::default()
    };
    handle_key(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
        &mut state,
    );
    assert!(!state.command_mode, "Esc must exit command mode");
    assert!(state.command_input.is_empty(), "Esc must clear input");
}

#[test]
fn handle_esc_closes_rich_popup() {
    use super::super::views::PopupContent;
    let mut state = InteractiveState {
        popup_open: true,
        popup_content: Some(PopupContent {
            title: "Detail".to_string(),
            sections: vec![],
            actions: vec![],
        }),
        ..Default::default()
    };
    handle_key(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
        &mut state,
    );
    assert!(!state.popup_open, "Esc must close rich popup");
    assert!(state.popup_content.is_none(), "Esc must clear popup content");
}

#[test]
fn handle_esc_clears_plan_filter() {
    let mut state = InteractiveState {
        selected_plan_id: Some(708),
        ..Default::default()
    };
    handle_key(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::NONE,
        &mut state,
    );
    assert!(state.selected_plan_id.is_none(), "Esc must clear plan filter");
}

#[test]
fn parse_command_plan_sets_plan_filter() {
    let mut state = InteractiveState::default();
    let mut view = MainView::PlanKanban;
    parse_and_apply_command("plan 708", &mut state, &mut view);
    assert_eq!(state.selected_plan_id, Some(708));
    assert_eq!(view, MainView::TaskPipeline);
}

#[test]
fn parse_command_mesh_switches_view() {
    let mut state = InteractiveState::default();
    let mut view = MainView::PlanKanban;
    parse_and_apply_command("mesh", &mut state, &mut view);
    assert_eq!(view, MainView::MeshStatus);
}

#[test]
fn parse_command_agent_list_switches_view() {
    let mut state = InteractiveState::default();
    let mut view = MainView::PlanKanban;
    parse_and_apply_command("agent list", &mut state, &mut view);
    assert_eq!(view, MainView::AgentOrgChart);
}

#[test]
fn parse_command_refresh_sets_force_refresh() {
    let mut state = InteractiveState::default();
    let mut view = MainView::PlanKanban;
    parse_and_apply_command("refresh", &mut state, &mut view);
    assert!(state.force_refresh, "refresh command must set force_refresh flag");
}

#[test]
fn parse_command_quit_sets_quit_flag() {
    let mut state = InteractiveState::default();
    let mut view = MainView::PlanKanban;
    parse_and_apply_command("quit", &mut state, &mut view);
    assert!(state.quit, "quit command must set quit flag");
    let mut state2 = InteractiveState::default();
    parse_and_apply_command("q", &mut state2, &mut view);
    assert!(state2.quit, "q command must set quit flag");
}

// --- Refresh controls ---

#[test]
fn auto_refresh_defaults_to_true() {
    // TuiApp::parse_auto_refresh_defaults() is a pure fn we can test without I/O
    assert!(TuiApp::default_auto_refresh(), "auto_refresh must default to true");
}

#[test]
fn default_refresh_interval_is_5s() {
    assert_eq!(TuiApp::default_refresh_interval_secs(), 5u64);
}

#[test]
fn shift_r_toggles_auto_refresh() {
    let mut state = InteractiveState::default();
    // InteractiveState tracks auto_refresh toggle request
    // shift+R sends KeyCode::Char('R') with SHIFT modifier
    handle_key(
        crossterm::event::KeyCode::Char('R'),
        crossterm::event::KeyModifiers::SHIFT,
        &mut state,
    );
    assert!(state.toggle_auto_refresh, "shift+R must set toggle_auto_refresh flag");
}

#[test]
fn plus_key_increases_interval() {
    let mut state = InteractiveState::default();
    handle_key(
        crossterm::event::KeyCode::Char('+'),
        crossterm::event::KeyModifiers::NONE,
        &mut state,
    );
    assert!(state.increase_interval, "'+' must set increase_interval flag");
}

#[test]
fn minus_key_decreases_interval() {
    let mut state = InteractiveState::default();
    handle_key(
        crossterm::event::KeyCode::Char('-'),
        crossterm::event::KeyModifiers::NONE,
        &mut state,
    );
    assert!(state.decrease_interval, "'-' must set decrease_interval flag");
}

#[test]
fn interval_steps_are_correct() {
    // Steps: 3, 5, 10, 30, 60 seconds
    assert_eq!(TuiApp::next_interval(3), 5);
    assert_eq!(TuiApp::next_interval(5), 10);
    assert_eq!(TuiApp::next_interval(10), 30);
    assert_eq!(TuiApp::next_interval(30), 60);
    assert_eq!(TuiApp::next_interval(60), 60); // at max, stays at 60
    assert_eq!(TuiApp::prev_interval(60), 30);
    assert_eq!(TuiApp::prev_interval(30), 10);
    assert_eq!(TuiApp::prev_interval(10), 5);
    assert_eq!(TuiApp::prev_interval(5), 3);
    assert_eq!(TuiApp::prev_interval(3), 3); // at min, stays at 3
}
