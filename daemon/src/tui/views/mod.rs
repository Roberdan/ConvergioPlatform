use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use super::{MainView, PlanHierarchyContext, TuiData};
use crate::tui::widgets::{self, ACCENT, MUTED, OK, TEXT_PRIMARY};

pub mod brain;
pub mod chat;
pub(super) mod chat_render;
#[cfg(test)]
mod chat_tests;
pub mod cost;
pub mod deliverables;
pub mod dep_graph;
pub mod events;
pub mod help;
pub mod hierarchy_bar;
pub mod popup;
pub mod project_switcher;
pub mod project_tree;
pub mod workspace;

pub use popup::{render_rich_popup, PopupContent};

const ALL_VIEWS: &[(MainView, &str)] = &[
    (MainView::PlanKanban, "Tree"),
    (MainView::Chat, "◆ Chat"),
    (MainView::TaskPipeline, "Pipeline"),
    (MainView::MeshStatus, "Mesh"),
    (MainView::AgentOrgChart, "Agents"),
    (MainView::BrainCanvas, "Brain"),
    (MainView::CostCenter, "Cost"),
    (MainView::EventStream, "Events"),
    (MainView::WorkspaceView, "WS"),
    (MainView::Deliverables, "Deliv"),
];

/// Renders tab bar, KPI strip, active view, status bar, and optional overlays.
///
/// If `popup_content` is Some, a rich popup is rendered as the topmost overlay.
/// If `show_help` is true, the help overlay is rendered (below popup in z-order).
/// If `show_project_switcher` is true, the project switcher overlay is rendered.
#[allow(clippy::too_many_arguments)]
pub fn render_view(
    frame: &mut Frame<'_>,
    area: Rect,
    view: MainView,
    data: &TuiData,
    selected: usize,
    api_url: &str,
    show_help: bool,
    auto_refresh: bool,
    refresh_interval_secs: u64,
    chat_input: &str,
    chat_sending: bool,
    popup_content: Option<&PopupContent>,
    show_all_plans: bool,
    chat_scroll: u16,
    expanded_masters: &[i64],
    hierarchy_context: Option<&PlanHierarchyContext>,
    show_project_switcher: bool,
    project_switcher_selected: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tab bar
            Constraint::Length(3), // KPI strip
            Constraint::Min(1),    // content
            Constraint::Length(3), // status bar
        ])
        .split(area);

    render_tab_bar(frame, chunks[0], view, &data.active_project_name);
    frame.render_widget(widgets::kpi_strip(data), chunks[1]);
    render_content(frame, chunks[2], view, data, selected, chat_input, chat_sending, show_all_plans, chat_scroll, expanded_masters, hierarchy_context);
    let unread = data.notifications.iter().filter(|n| !n.read).count();
    render_status_bar(frame, chunks[3], api_url, auto_refresh, refresh_interval_secs, unread);

    if show_help {
        help::render_help_overlay(frame, area);
    }

    // Project switcher overlay (below rich popup in z-order).
    if show_project_switcher {
        project_switcher::render_project_switcher(
            frame, area, &data.projects, project_switcher_selected,
        );
    }

    // Rich popup renders last (topmost overlay).
    if let Some(content) = popup_content {
        render_rich_popup(frame, area, content);
    }
}

// --- Tab bar ---

fn render_tab_bar(frame: &mut Frame<'_>, area: Rect, active: MainView, project_name: &str) {
    // Show active project name after the logo; fall back to "Convergio" when unset.
    let display_project = if project_name.is_empty() { "Convergio" } else { project_name };
    let header = format!(" ◆ {}  ", display_project);
    let mut spans: Vec<Span<'static>> = vec![Span::styled(
        header,
        Style::default().fg(ACCENT).bold(),
    )];

    for (view, label) in ALL_VIEWS {
        let sep = Span::styled("│", Style::default().fg(MUTED));
        spans.push(sep);
        if *view == active {
            spans.push(Span::styled(
                format!(" {} ", label),
                Style::default().fg(ACCENT).bold().reversed(),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {} ", label),
                Style::default().fg(MUTED),
            ));
        }
    }

    let paragraph = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );
    frame.render_widget(paragraph, area);
}

// --- Content dispatch ---

#[allow(clippy::too_many_arguments)]
fn render_content(
    frame: &mut Frame<'_>,
    area: Rect,
    view: MainView,
    data: &TuiData,
    selected: usize,
    chat_input: &str,
    chat_sending: bool,
    show_all_plans: bool,
    chat_scroll: u16,
    expanded_masters: &[i64],
    hierarchy_context: Option<&PlanHierarchyContext>,
) {
    match view {
        MainView::PlanKanban => {
            if data.project_tree.plans.is_empty() {
                frame.render_widget(widgets::plan_kanban(data, selected, show_all_plans), area);
            } else {
                frame.render_widget(
                    project_tree::project_tree_view(data, selected, expanded_masters), area,
                );
            }
        }
        MainView::TaskPipeline => {
            // If a hierarchy context exists, split area: 3-line bar on top, pipeline below.
            if let Some(ctx) = hierarchy_context {
                let splits = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(1)])
                    .split(area);
                hierarchy_bar::render_hierarchy_bar(frame, splits[0], ctx);
                frame.render_widget(widgets::task_pipeline(data, selected), splits[1]);
            } else {
                frame.render_widget(widgets::task_pipeline(data, selected), area);
            }
        }
        MainView::MeshStatus => {
            frame.render_widget(widgets::mesh_status(data, selected), area);
        }
        MainView::AgentOrgChart => {
            frame.render_widget(widgets::agent_org_chart(data, selected), area);
        }
        MainView::BrainCanvas => {
            frame.render_widget(brain::brain_canvas(data, selected), area);
        }
        MainView::CostCenter => {
            frame.render_widget(cost::cost_center(data, selected), area);
        }
        MainView::EventStream => {
            frame.render_widget(events::event_stream(data, selected), area);
        }
        MainView::WorkspaceView => {
            frame.render_widget(workspace::workspace_view(data, selected), area);
        }
        MainView::Deliverables => {
            frame.render_widget(deliverables::deliverables_view(data, selected), area);
        }
        MainView::Chat => {
            chat::render_chat_view(frame, area, data, chat_input, chat_sending, chat_scroll);
        }
        MainView::ProjectView => {
            frame.render_widget(widgets::project_list(data, selected), area);
        }
    }
}

pub(crate) mod status_bar;
pub use status_bar::{render_command_footer, render_status_bar};
