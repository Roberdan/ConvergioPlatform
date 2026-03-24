// Refresh control: auto/manual toggle, interval stepping, post-key processing.

use super::app::TuiApp;
use super::chat_handler;
use super::data::MainView;
use super::views::popup::{PopupContent, PopupSection};

impl TuiApp {
    /// Drill-down: Enter on Kanban -> filter tasks; on Pipeline -> show detail.
    /// In Chat view, Enter triggers a send (async portion handled in process_post_key).
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
                    self.istate.popup_content = Some(PopupContent {
                        title: "Task Detail".to_string(),
                        sections: vec![
                            PopupSection {
                                label: "Identity".to_string(),
                                lines: vec![
                                    format!("task_id: {}", task.task_id),
                                    format!("title:   {}", task.title),
                                ],
                            },
                            PopupSection {
                                label: "Status".to_string(),
                                lines: vec![
                                    format!("status: {}", task.status),
                                    format!("agent:  {}", task.agent),
                                ],
                            },
                        ],
                        actions: vec![],
                    });
                    self.istate.popup_open = true;
                }
            }
            MainView::Chat => {
                // Signal that a chat send is pending; async work done in process_post_key.
                if !self.chat.input.is_empty() && !self.chat.sending {
                    self.chat.sending = true;
                }
            }
            _ => {}
        }
    }

    /// Post-key: consume pending flags. Returns true if interval changed.
    pub async fn process_post_key(&mut self) -> bool {
        // Chat send: triggered when chat.sending=true and input is non-empty.
        if self.chat.sending && !self.chat.input.is_empty() {
            let content = std::mem::take(&mut self.chat.input);
            chat_handler::push_user_message(&mut self.data, &content);
            // Temporarily move content back so send_message can read it.
            self.chat.input = content;
            let client = self.http_client().clone();
            chat_handler::send_message(
                &client,
                &self.api_url,
                &mut self.data,
                &mut self.chat,
            ).await;
            self.chat.input.clear();
            // Apply the pending reply if present.
            if let Some(result) = self.chat.pending_reply.take() {
                match result {
                    Ok(reply) => chat_handler::push_assistant_message(&mut self.data, &reply),
                    Err(()) => chat_handler::push_assistant_message(
                        &mut self.data,
                        "(No response — check daemon connection)",
                    ),
                }
            }
        }

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
