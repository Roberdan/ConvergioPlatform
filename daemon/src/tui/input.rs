// TUI input handling — key dispatch, command parsing, interactive state.
// Extracted from app.rs to keep app.rs under 250 lines.

use crossterm::event::{KeyCode, KeyModifiers};

use super::data::MainView;

/// All mutable interactive state owned by TuiApp.
#[derive(Default)]
pub struct InteractiveState {
    /// Which plan to filter tasks by (drill-down from Kanban).
    pub selected_plan_id: Option<i64>,
    /// Detail text for the popup overlay (task or agent detail).
    pub detail_text: Option<String>,
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
}

/// Handle a single key event; mutates state. Returns true if the app should quit.
/// Priority order: command_mode → help overlay → normal keys.
pub fn handle_key(code: KeyCode, modifiers: KeyModifiers, state: &mut InteractiveState) -> bool {
    if state.command_mode {
        handle_command_key(code, state);
        return state.quit;
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

/// Handle Esc outside command mode: close popups/filters in priority order.
fn handle_esc(state: &mut InteractiveState) {
    if state.detail_text.is_some() {
        state.detail_text = None;
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

    #[test]
    fn esc_in_command_mode_takes_priority() {
        let mut s = InteractiveState {
            command_mode: true,
            command_input: "abc".into(),
            detail_text: Some("x".into()),
            ..Default::default()
        };
        handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut s);
        // command mode exited; detail_text not touched (command mode had priority)
        assert!(!s.command_mode);
        assert!(s.detail_text.is_some(), "detail popup should survive if command mode exits first");
    }

    #[test]
    fn r_key_sets_force_refresh() {
        let mut s = InteractiveState::default();
        handle_key(KeyCode::Char('r'), KeyModifiers::NONE, &mut s);
        assert!(s.force_refresh);
    }
}
