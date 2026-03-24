// Project tree navigation — Enter handling for master expand/collapse and child drill-down.

use super::app::TuiApp;
use super::data::MainView;

impl TuiApp {
    /// Handle Enter in project tree view: toggle master expand or drill into child plan.
    pub(crate) fn handle_tree_enter(&mut self) {
        let tree = &self.data.project_tree;
        let expanded = &self.istate.expanded_masters;
        let mut idx: usize = 0;
        let (masters, orphans): (Vec<_>, Vec<_>) =
            tree.plans.iter().partition(|p| p.is_master);
        for master in &masters {
            if idx == self.selected_index {
                let mid = master.id;
                if let Some(pos) = self.istate.expanded_masters.iter().position(|&id| id == mid) {
                    self.istate.expanded_masters.remove(pos);
                } else {
                    self.istate.expanded_masters.push(mid);
                }
                return;
            }
            idx += 1;
            if expanded.contains(&master.id) {
                for child in &master.children {
                    if idx == self.selected_index {
                        self.istate.selected_plan_id = Some(child.id);
                        self.active_view = MainView::TaskPipeline;
                        self.selected_index = 0;
                        return;
                    }
                    idx += 1;
                }
            }
        }
        for orphan in &orphans {
            if idx == self.selected_index {
                self.istate.selected_plan_id = Some(orphan.id);
                self.active_view = MainView::TaskPipeline;
                self.selected_index = 0;
                return;
            }
            idx += 1;
        }
    }
}
