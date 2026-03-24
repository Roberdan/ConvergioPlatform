// Refresh control: auto/manual toggle, interval stepping, post-key processing.

use super::api::detail::{
    fetch_agent_detail, fetch_deliverable_detail, fetch_node_detail, fetch_plan_detail,
    fetch_task_detail, fetch_workspace_detail, format_event_detail,
};
use super::api::{mesh_heartbeat, mesh_provision, stop_agent};
use super::app::TuiApp;
use super::chat_handler;
use super::data::MainView;
use super::drill_down::DrillDownRequest;
use super::views::popup::{PopupContent, PopupSection};

impl TuiApp {
    /// Drill-down: Enter on Kanban -> filter tasks (navigation, no popup).
    /// On all other views, sets pending_drill_down for async resolution in process_post_key.
    /// Chat view triggers a send; BrainCanvas/CostCenter are no-ops.
    pub fn handle_enter(&mut self) {
        match self.active_view {
            MainView::PlanKanban => {
                // Navigate to TaskPipeline filtered by selected plan.
                if let Some(plan) = self.data.plans.get(self.selected_index) {
                    self.istate.selected_plan_id = Some(plan.id);
                    self.active_view = MainView::TaskPipeline;
                    self.selected_index = 0;
                }
            }
            MainView::TaskPipeline => {
                if self.data.pipeline.get(self.selected_index).is_some() {
                    let plan_id = self.istate.selected_plan_id.unwrap_or(0);
                    self.istate.pending_drill_down =
                        Some(DrillDownRequest::Task(plan_id, self.selected_index));
                }
            }
            MainView::MeshStatus => {
                if let Some(node) = self.data.mesh_nodes.get(self.selected_index) {
                    self.istate.pending_drill_down =
                        Some(DrillDownRequest::MeshNode(node.name.clone()));
                }
            }
            MainView::AgentOrgChart => {
                if let Some(agent) = self.data.agents.get(self.selected_index) {
                    self.istate.pending_drill_down =
                        Some(DrillDownRequest::Agent(agent.name.clone()));
                }
            }
            MainView::EventStream => {
                if self.data.events.get(self.selected_index).is_some() {
                    self.istate.pending_drill_down =
                        Some(DrillDownRequest::Event(self.selected_index));
                }
            }
            MainView::WorkspaceView => {
                if let Some(ws) = self.data.workspaces.get(self.selected_index) {
                    self.istate.pending_drill_down =
                        Some(DrillDownRequest::Workspace(ws.workspace_id.clone()));
                }
            }
            MainView::Deliverables => {
                if let Some(d) = self.data.deliverables.get(self.selected_index) {
                    self.istate.pending_drill_down =
                        Some(DrillDownRequest::Deliverable(d.id));
                }
            }
            MainView::Chat => {
                // Signal that a chat send is pending; async work done in process_post_key.
                if !self.chat.input.is_empty() && !self.chat.sending {
                    self.chat.sending = true;
                }
            }
            // BrainCanvas and CostCenter: no drill-down defined.
            MainView::BrainCanvas | MainView::CostCenter => {}
        }
    }

    /// Post-key: consume pending flags. Returns true if interval changed.
    pub async fn process_post_key(&mut self) -> bool {
        // Drill-down: resolve pending_drill_down set by handle_enter.
        if let Some(req) = self.istate.pending_drill_down.take() {
            let client = self.http_client().clone();
            let api_url = self.api_url.clone();
            let content = match req {
                DrillDownRequest::Plan(plan_id) => {
                    fetch_plan_detail(&client, &api_url, plan_id).await
                }
                DrillDownRequest::Task(plan_id, task_idx) => {
                    fetch_task_detail(&client, &api_url, plan_id, task_idx).await
                }
                DrillDownRequest::MeshNode(ref name) => {
                    fetch_node_detail(&client, &api_url, name).await
                }
                DrillDownRequest::Agent(ref name) => {
                    fetch_agent_detail(&client, &api_url, name).await
                }
                DrillDownRequest::Event(idx) => {
                    // Sync — no API call needed; clone the event before borrow ends.
                    if let Some(event) = self.data.events.get(idx).cloned() {
                        format_event_detail(&event)
                    } else {
                        use super::views::popup::{PopupContent, PopupSection};
                        PopupContent {
                            title: "Error".to_string(),
                            sections: vec![PopupSection {
                                label: "Error".to_string(),
                                lines: vec![format!("event index {idx} not found")],
                            }],
                            actions: vec![],
                        }
                    }
                }
                DrillDownRequest::Workspace(ref ws_id) => {
                    fetch_workspace_detail(&client, &api_url, ws_id).await
                }
                DrillDownRequest::Deliverable(id) => {
                    fetch_deliverable_detail(&client, &api_url, id).await
                }
            };
            self.istate.popup_content = Some(content);
            self.istate.popup_open = true;
        }

        // Action: consume action_pending set by popup key handler.
        if let Some((action, target)) = self.istate.action_pending.take() {
            let client = self.http_client().clone();
            let api_url = self.api_url.clone();
            let result = match action.as_str() {
                "Provision" => mesh_provision(&client, &api_url, &target).await,
                "Heartbeat" => mesh_heartbeat(&client, &api_url).await,
                "Stop Agent" => {
                    let output = stop_agent(&client, &api_url, &target).await;
                    // Force data refresh so agent list reflects the stop.
                    self.istate.force_refresh = true;
                    output
                }
                other => format!("Unknown action: {other}"),
            };
            // Show result in a new popup.
            self.istate.popup_content = Some(PopupContent {
                title: format!("{action} Result"),
                sections: vec![PopupSection {
                    label: "Output".to_string(),
                    lines: vec![result],
                }],
                actions: vec![],
            });
            self.istate.popup_open = true;
        }

        // Chat send: push user message, send through persistent session.
        if self.chat.sending && !self.chat.input.is_empty() {
            let content = std::mem::take(&mut self.chat.input);
            tracing::info!(content = %content, "chat: user message via persistent session");
            chat_handler::push_user_message(&mut self.data, &content);
            if self.chat.send_to_session(&content).await {
                self.chat.streaming = true;
                // Add empty assistant message that will be filled by streaming.
                chat_handler::push_assistant_message(&mut self.data, "");
            } else {
                chat_handler::push_assistant_message(
                    &mut self.data, "(Failed to send — claude CLI not available)",
                );
            }
            self.chat.sending = false;
        }
        // Poll streaming events from persistent session (non-blocking).
        {
            use crate::tui::claude_session::ChatEvent;
            let events = self.chat.poll_events();
            for event in events {
                match event {
                    ChatEvent::TextDelta(delta) => {
                        // Append delta to the last assistant message.
                        if let Some(last) = self.data.chat_messages.last_mut() {
                            if last.role == "assistant" {
                                last.content.push_str(&delta);
                            }
                        }
                    }
                    ChatEvent::MessageComplete(text) => {
                        tracing::info!("chat: Ali response complete");
                        // Replace last assistant message with complete text.
                        if let Some(last) = self.data.chat_messages.last_mut() {
                            if last.role == "assistant" {
                                last.content = text;
                            }
                        }
                        self.chat.streaming = false;
                    }
                    ChatEvent::SessionReady(sid) => {
                        tracing::info!(session_id = %sid, "chat: session ready");
                        self.data.chat_session_id = Some(sid);
                    }
                    ChatEvent::Error(msg) => {
                        tracing::warn!(error = %msg, "chat: session error");
                        if self.chat.streaming {
                            chat_handler::push_assistant_message(
                                &mut self.data, &format!("(Error: {msg})"),
                            );
                            self.chat.streaming = false;
                        }
                    }
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
