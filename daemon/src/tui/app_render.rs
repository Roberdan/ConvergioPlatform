// TuiApp rendering and view navigation — extracted from app.rs (250-line limit).

use super::app::TuiApp;
use super::data::MainView;
use super::views;
use std::io;

impl TuiApp {
    pub(crate) fn render(&mut self) -> io::Result<()> {
        let view = self.active_view;
        let data = &self.data;
        let selected = self.selected_index;
        let api_url = self.api_url.clone();
        let show_help = self.istate.show_help;
        let auto_refresh = self.auto_refresh;
        let refresh_interval_secs = self.refresh_interval_secs;
        let cmd_input = self
            .istate
            .command_mode
            .then(|| self.istate.command_input.clone());
        let chat_input = self.chat.input.clone();
        let chat_sending = self.chat.sending || self.chat.streaming;
        let chat_scroll = self.chat.scroll_offset;
        let popup_content = self.istate.popup_open.then(|| self.istate.popup_content.clone()).flatten();
        let show_all_plans = self.istate.show_all_plans;
        let expanded_masters = self.istate.expanded_masters.clone();
        let hierarchy_context = self.istate.hierarchy_context.clone();
        let show_project_switcher = self.istate.show_project_switcher;
        let project_switcher_selected = self.istate.project_switcher_selected;
        self.terminal.draw(|frame| {
            views::render_view(
                frame, frame.area(), view, data, selected, &api_url,
                show_help, auto_refresh, refresh_interval_secs,
                &chat_input, chat_sending, popup_content.as_ref(), show_all_plans,
                chat_scroll, &expanded_masters, hierarchy_context.as_ref(),
                show_project_switcher, project_switcher_selected,
            );
            // Rich popup overlay (if open)
            if let Some(ref pc) = popup_content {
                views::popup::render_rich_popup(frame, frame.area(), pc);
            }
            if cmd_input.is_some() {
                let area = frame.area();
                let fh = 3_u16.min(area.height);
                let footer = ratatui::layout::Rect {
                    x: area.x,
                    y: area.y + area.height.saturating_sub(fh),
                    width: area.width,
                    height: fh,
                };
                views::render_command_footer(frame, footer, cmd_input.as_deref());
            }
        })?;
        Ok(())
    }

    pub fn next_view(&mut self) {
        self.selected_index = 0;
        let idx = Self::view_index(self.active_view);
        self.active_view = Self::view_at((idx + 1) % 11);
    }

    pub fn prev_view(&mut self) {
        self.selected_index = 0;
        let idx = Self::view_index(self.active_view);
        self.active_view = Self::view_at((idx + 10) % 11);
    }

    fn view_index(v: MainView) -> usize {
        use MainView::*;
        [
            PlanKanban, Chat, TaskPipeline, MeshStatus, AgentOrgChart,
            BrainCanvas, CostCenter, EventStream, WorkspaceView, Deliverables,
            ProjectView,
        ]
        .iter()
        .position(|x| *x == v)
        .unwrap_or(0)
    }

    fn view_at(idx: usize) -> MainView {
        use MainView::*;
        [
            PlanKanban, Chat, TaskPipeline, MeshStatus, AgentOrgChart,
            BrainCanvas, CostCenter, EventStream, WorkspaceView, Deliverables,
            ProjectView,
        ]
        .get(idx)
        .copied()
        .unwrap_or(PlanKanban)
    }
}
