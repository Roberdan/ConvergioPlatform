// Refresh control: auto/manual toggle, interval stepping, post-key processing.

use super::app::TuiApp;
use super::data::MainView;
use super::input;

impl TuiApp {
    /// Drill-down: Enter on Kanban -> filter tasks; on Pipeline -> show detail.
    pub fn handle_enter(&mut self) {
        match self.active_view {
            MainView::PlanKanban => {
                if let Some(plan) = self.data.plans.get(self.selected_index) {
                    self.istate.selected_plan_id = Some(plan.id);
                    self.active_view = MainView::TaskPipeline;
                    self.selected_index = 0;
                }
            }
            MainView::TaskPipeline => {
                if let Some(task) = self.data.pipeline.get(self.selected_index) {
                    self.istate.detail_text = Some(format!(
                        "task_id: {}\ntitle: {}\nstatus: {}\nagent: {}",
                        task.task_id, task.title, task.status, task.agent
                    ));
                }
            }
            _ => {}
        }
    }

    /// Post-key: consume pending flags. Returns true if interval changed.
    pub async fn process_post_key(&mut self) -> bool {
        if self.istate.force_refresh {
            self.istate.force_refresh = false;
            self.refresh_data().await;
        }
        if self.istate.toggle_auto_refresh {
            self.istate.toggle_auto_refresh = false;
            self.auto_refresh = !self.auto_refresh;
        }
        let mut changed = false;
        if self.istate.increase_interval {
            self.istate.increase_interval = false;
            let nv = Self::next_interval(self.refresh_interval_secs);
            if nv != self.refresh_interval_secs { self.refresh_interval_secs = nv; changed = true; }
        }
        if self.istate.decrease_interval {
            self.istate.decrease_interval = false;
            let nv = Self::prev_interval(self.refresh_interval_secs);
            if nv != self.refresh_interval_secs { self.refresh_interval_secs = nv; changed = true; }
        }
        changed
    }

    pub fn default_auto_refresh() -> bool { true }
    pub fn default_refresh_interval_secs() -> u64 { 5 }

    pub fn next_interval(current: u64) -> u64 {
        match current { 0..=3 => 5, 4..=5 => 10, 6..=10 => 30, 11..=30 => 60, _ => 60 }
    }

    pub fn prev_interval(current: u64) -> u64 {
        match current { 0..=3 => 3, 4..=5 => 3, 6..=10 => 5, 11..=30 => 10, 31..=60 => 30, _ => 30 }
    }
}
