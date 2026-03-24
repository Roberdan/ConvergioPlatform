// TUI input handling — key dispatch, command parsing, interactive state.
// Extracted from app.rs to keep app.rs under 250 lines.

use crossterm::event::{KeyCode, KeyModifiers};

use super::data::MainView;
use super::views::PopupContent;

pub use super::drill_down::DrillDownRequest;

/// All mutable interactive state owned by TuiApp.
#[derive(Default)]
pub struct InteractiveState {
    /// Which plan to filter tasks by (drill-down from Kanban).
    pub selected_plan_id: Option<i64>,
    /// Current command being typed (activated by `/`).
    pub command_input: String,
    /// Whether the command input bar is active.
    pub command_mode: bool,
    /// Whether a force-refresh was requested (consumed by run loop).
    pub force_refresh: bool,
    /// Whether the app should quit.
    pub quit: bool,
    /// Whether to show the help overlay.
    pub show_help: bool,
    /// Request to toggle auto-refresh on/off (consumed by run loop).
    pub toggle_auto_refresh: bool,
    /// Request to increase refresh interval (consumed by run loop).
    pub increase_interval: bool,
    /// Request to decrease refresh interval (consumed by run loop).
    pub decrease_interval: bool,
    /// Whether the rich popup overlay is open.
    pub popup_open: bool,
    /// Content for the rich popup overlay (set before opening).
    pub popup_content: Option<PopupContent>,
    /// Pending action triggered by a popup action key: (action_name, target_id).
    pub action_pending: Option<(String, String)>,
    /// Show all plans in kanban (including done). Toggle with 'a'.
    pub show_all_plans: bool,
    /// Pending drill-down request set synchronously by handle_enter.
    /// Resolved asynchronously in process_post_key by calling the appropriate api/detail fetch.
    pub pending_drill_down: Option<DrillDownRequest>,
}

/// Handle a single key event; mutates state. Returns true if the app should quit.
/// Priority order: command_mode → popup_open → help overlay → normal keys.
pub fn handle_key(code: KeyCode, modifiers: KeyModifiers, state: &mut InteractiveState) -> bool {
    if state.command_mode {
        handle_command_key(code, state);
        return state.quit;
    }

    // Popup captures all focus; only Esc and action keys are handled.
    if state.popup_open {
        handle_popup_key(code, state);
        return false;
    }

    if state.show_help {
        match code {
            KeyCode::Char('?') | KeyCode::Esc => state.show_help = false,
            _ => {}
        }
        return false;
    }

    match code {
        KeyCode::Char('q') => return true,
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
        KeyCode::Char('/') => {
            state.command_mode = true;
            state.command_input.clear();
        }
        KeyCode::Char('r') => state.force_refresh = true,
        KeyCode::Char('a') => state.show_all_plans = !state.show_all_plans,
        // Shift+R toggles auto-refresh on/off.
        KeyCode::Char('R') => state.toggle_auto_refresh = true,
        // +/- adjust refresh interval.
        KeyCode::Char('+') | KeyCode::Char('=') => state.increase_interval = true,
        KeyCode::Char('-') => state.decrease_interval = true,
        KeyCode::Char('?') => state.show_help = true,
        KeyCode::Esc => handle_esc(state),
        _ => {}
    }
    false
}

/// Handle a key while in command mode.
fn handle_command_key(code: KeyCode, state: &mut InteractiveState) {
    match code {
        KeyCode::Esc => {
            state.command_mode = false;
            state.command_input.clear();
        }
        KeyCode::Backspace => {
            state.command_input.pop();
        }
        KeyCode::Char(c) => {
            state.command_input.push(c);
        }
        KeyCode::Enter => {
            let cmd = std::mem::take(&mut state.command_input);
            state.command_mode = false;
            // Apply via caller; store pending command so app.rs can read it.
            state.command_input = cmd;
            state.force_refresh = false; // reset; parse_and_apply_command sets it
        }
        _ => {}
    }
}

/// Handle keys when the rich popup is open.
/// Esc closes the popup; action keys set action_pending; all other keys are swallowed.
fn handle_popup_key(code: KeyCode, state: &mut InteractiveState) {
    match code {
        KeyCode::Esc => {
            state.popup_open = false;
            state.popup_content = None;
        }
        KeyCode::Char(c) => {
            // Check if this char matches an action key in the current popup.
            let action = state
                .popup_content
                .as_ref()
                .and_then(|p| p.action_for_key(c))
                .map(str::to_string);
            if let Some(label) = action {
                // Store action for the run loop to consume; target_id is empty by default.
                // Callers that open the popup can pre-populate action_pending target_id
                // after the fact, or the popup can be extended with a target field.
                state.action_pending = Some((label, String::new()));
                // Close popup after action is selected.
                state.popup_open = false;
                state.popup_content = None;
            }
            // Unknown keys are ignored (popup captures focus).
        }
        _ => {}
    }
}

/// Handle Esc outside command/popup mode: close overlays/filters in priority order.
fn handle_esc(state: &mut InteractiveState) {
    if state.popup_open {
        state.popup_open = false;
        state.popup_content = None;
    } else if state.selected_plan_id.is_some() {
        state.selected_plan_id = None;
    }
    // If nothing to close, Esc is a no-op.
}

/// Parse and apply a command string (called after Enter in command mode).
/// View is passed mutably so commands can switch view.
pub fn parse_and_apply_command(
    cmd: &str,
    state: &mut InteractiveState,
    view: &mut MainView,
) {
    let parts: Vec<&str> = cmd.trim().splitn(2, ' ').collect();
    match parts.as_slice() {
        ["plan", id] => {
            if let Ok(n) = id.trim().parse::<i64>() {
                state.selected_plan_id = Some(n);
                *view = MainView::TaskPipeline;
            }
        }
        ["mesh"] => *view = MainView::MeshStatus,
        ["agent", "list"] | ["agent"] => *view = MainView::AgentOrgChart,
        ["refresh"] => state.force_refresh = true,
        ["quit"] | ["q"] => state.quit = true,
        _ => {}
    }
}

#[cfg(test)]
mod unit {
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
        // command_mode has highest priority; popup_open is not touched.
        let mut s = InteractiveState {
            command_mode: true,
            command_input: "abc".into(),
            popup_open: true,
            popup_content: Some(make_popup()),
            ..Default::default()
        };
        handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut s);
        // command mode exited; popup not touched (command mode had priority)
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
        // 'q' would normally quit, but popup captures focus.
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
}
