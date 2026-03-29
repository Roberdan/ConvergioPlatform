// Command parsing — called after Enter in command mode.
use super::{InteractiveState, MainView};

/// Parse and apply a command string. View is passed mutably so commands can switch view.
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
        ["projects"] | ["project"] => *view = MainView::ProjectView,
        ["refresh"] => state.force_refresh = true,
        ["quit"] | ["q"] => state.quit = true,
        _ => {}
    }
}
