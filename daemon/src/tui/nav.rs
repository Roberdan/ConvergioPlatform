// Navigation helpers extracted from app.rs.
// Why: keep app.rs ≤250 lines per CONSTITUTION Article V.
use super::app::TuiApp;
use super::data::MainView;

pub fn switch_view(app: &mut TuiApp, n: u8) {
    use MainView::*;
    app.selected_index = 0;
    // 0 = Deliverables (10th), 1-9 = ordered views
    let views = [
        Deliverables, PlanKanban, Chat, TaskPipeline, MeshStatus,
        AgentOrgChart, BrainCanvas, CostCenter, EventStream, WorkspaceView,
    ];
    app.active_view = views[(n as usize).min(9)];
}

pub fn list_len(app: &TuiApp) -> usize {
    match app.active_view {
        MainView::PlanKanban => {
            if app.data.project_tree.plans.is_empty() {
                app.data.plans.len()
            } else {
                crate::tui::views::project_tree::build_tree_lines(
                    &app.data.project_tree,
                    app.selected_index,
                    &app.istate.expanded_masters,
                )
                .1
            }
        }
        MainView::TaskPipeline => app.data.pipeline.len(),
        MainView::MeshStatus => app.data.mesh_nodes.len(),
        MainView::AgentOrgChart => app.data.agents.len(),
        MainView::BrainCanvas => app.data.brain_nodes.len(),
        MainView::CostCenter => app.data.cost.by_model.len(),
        MainView::EventStream => app.data.events.len(),
        MainView::WorkspaceView => app.data.workspaces.len(),
        MainView::Deliverables => app.data.deliverables.len(),
        MainView::Chat => app.data.chat_messages.len(),
        MainView::ProjectView => app.data.projects.len(),
    }
}
