// TUI input handling — key dispatch, command parsing, interactive state.
use crossterm::event::{KeyCode, KeyModifiers};

use super::data::{MainView, PlanHierarchyContext};
use super::views::PopupContent;

pub use super::drill_down::DrillDownRequest;

#[derive(Default)]
pub struct InteractiveState {
    pub selected_plan_id: Option<i64>,
    pub command_input: String,
    pub command_mode: bool,
    pub force_refresh: bool,
    pub quit: bool,
    pub show_help: bool,
    pub toggle_auto_refresh: bool,
    pub increase_interval: bool,
    pub decrease_interval: bool,
    pub popup_open: bool,
    pub popup_content: Option<PopupContent>,
    pub action_pending: Option<(String, String)>,
    pub show_all_plans: bool,
    pub pending_drill_down: Option<DrillDownRequest>,
    /// Whether the notification inbox overlay is visible.
    pub show_notifications: bool,
    /// Index of selected notification in the inbox overlay.
    pub notification_selected: usize,
    /// IDs of expanded master plans in the project tree view.
    pub expanded_masters: Vec<i64>,
    /// Set to true when 'p' is pressed to switch to the Project view.
    pub switch_to_project_view: bool,
    /// Hierarchy context populated when drilling into a child plan from the project tree.
    pub hierarchy_context: Option<PlanHierarchyContext>,
    /// Whether the Ctrl+P project switcher overlay is visible.
    pub show_project_switcher: bool,
    /// Currently highlighted index in the project switcher list.
    pub project_switcher_selected: usize,
    /// Index of the project to switch to (set on Enter, cleared after app processes it).
    pub pending_project_switch: Option<String>,
}

/// Handle a single key event; mutates state. Returns true if the app should quit.
/// Priority order: command_mode → popup_open → project_switcher → help overlay → normal keys.
pub fn handle_key(code: KeyCode, modifiers: KeyModifiers, state: &mut InteractiveState) -> bool {
    // Ctrl+P always toggles the project switcher (even from other modes).
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('p') {
        state.show_project_switcher = !state.show_project_switcher;
        if state.show_project_switcher {
            state.project_switcher_selected = 0;
        }
        return false;
    }

    if state.command_mode {
        handle_command_key(code, state);
        return state.quit;
    }

    // Popup captures all focus; only Esc and action keys are handled.
    if state.popup_open {
        handle_popup_key(code, state);
        return false;
    }

    // Project switcher captures Up/Down/Enter/Esc when open.
    if state.show_project_switcher {
        handle_project_switcher_key(code, state);
        return false;
    }

    // Notification inbox captures focus when open.
    if state.show_notifications {
        handle_notification_key(code, state);
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
        KeyCode::Char('p') => state.switch_to_project_view = true,
        // Shift+R toggles auto-refresh on/off.
        KeyCode::Char('R') => state.toggle_auto_refresh = true,
        // +/- adjust refresh interval.
        KeyCode::Char('+') | KeyCode::Char('=') => state.increase_interval = true,
        KeyCode::Char('-') => state.decrease_interval = true,
        KeyCode::Char('n') => state.show_notifications = !state.show_notifications,
        KeyCode::Char('o') => crate::tui::widgets::agents::toggle_org_hierarchy_mode(),
        KeyCode::Char('?') => state.show_help = true,
        KeyCode::Esc => handle_esc(state),
        _ => {}
    }
    false
}

/// Handle keys while project switcher is open.
fn handle_project_switcher_key(code: KeyCode, state: &mut InteractiveState) {
    match code {
        KeyCode::Esc => {
            state.show_project_switcher = false;
        }
        KeyCode::Up => {
            state.project_switcher_selected = state.project_switcher_selected.saturating_sub(1);
        }
        KeyCode::Down => {
            state.project_switcher_selected += 1;
        }
        KeyCode::Enter => {
            // Store the selected index as a string so app.rs can resolve it against the
            // projects list and switch context. Switcher closes immediately.
            state.pending_project_switch =
                Some(state.project_switcher_selected.to_string());
            state.show_project_switcher = false;
        }
        _ => {}
    }
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
                // Extract target from popup title (format "Prefix: target_name").
                // e.g. "Node: macProM1" → "macProM1", "Agent: Thor" → "Thor".
                let target = state
                    .popup_content
                    .as_ref()
                    .and_then(|p| p.title.split_once(": "))
                    .map(|(_, t)| t.to_string())
                    .unwrap_or_default();
                state.action_pending = Some((label, target));
                // Close popup after action is selected.
                state.popup_open = false;
                state.popup_content = None;
            }
            // Unknown keys are ignored (popup captures focus).
        }
        _ => {}
    }
}

/// Handle keys when the notification inbox overlay is open.
/// Esc or 'n' closes it; Up/Down navigate; Enter marks as read (signalled via flag).
fn handle_notification_key(code: KeyCode, state: &mut InteractiveState) {
    match code {
        KeyCode::Esc | KeyCode::Char('n') => {
            state.show_notifications = false;
        }
        KeyCode::Up => {
            state.notification_selected = state.notification_selected.saturating_sub(1);
        }
        KeyCode::Down => {
            state.notification_selected += 1; // clamped at render time
        }
        KeyCode::Enter => {
            // Mark-as-read handled by the caller (process_post_key).
            // We signal via the selected index remaining set.
        }
        _ => {} // swallow all other keys
    }
}

/// Handle Esc outside command/popup mode: close overlays/filters in priority order.
fn handle_esc(state: &mut InteractiveState) {
    if state.show_project_switcher {
        state.show_project_switcher = false;
    } else if state.popup_open {
        state.popup_open = false;
        state.popup_content = None;
    } else if state.show_notifications {
        state.show_notifications = false;
    } else if state.selected_plan_id.is_some() {
        state.selected_plan_id = None;
        // Clear hierarchy context when leaving the drill-down view.
        state.hierarchy_context = None;
    }
    // If nothing to close, Esc is a no-op.
}

// Command parsing → input_commands.rs
pub use input_commands::parse_and_apply_command;
mod input_commands;

#[cfg(test)]
mod unit;
