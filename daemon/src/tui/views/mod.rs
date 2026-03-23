use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::{MainView, TuiData};
use crate::tui::widgets;

pub mod brain;
pub mod cost;
pub mod events;

/// Renders header, KPI strip, active view with selection, and footer.
pub fn render_view(
    frame: &mut Frame<'_>,
    area: Rect,
    view: MainView,
    data: &TuiData,
    selected: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(area);

    let header = Paragraph::new(Line::from(format!(
        " Convergio Rust TUI | {} ",
        view_name(view)
    )))
    .block(Block::default().borders(Borders::ALL))
    .style(
        Style::default()
            .fg(Color::from_u32(widgets::ACCENT_U32))
            .bold(),
    );
    frame.render_widget(header, chunks[0]);

    frame.render_widget(widgets::kpi_strip(data), chunks[1]);

    match view {
        MainView::PlanKanban => {
            frame.render_widget(widgets::plan_kanban(data, selected), chunks[2]);
        }
        MainView::TaskPipeline => {
            frame.render_widget(widgets::task_pipeline(data, selected), chunks[2]);
        }
        MainView::MeshStatus => {
            frame.render_widget(widgets::mesh_status(data, selected), chunks[2]);
        }
        MainView::AgentOrgChart => {
            frame.render_widget(widgets::agent_org_chart(data, selected), chunks[2]);
        }
        MainView::BrainCanvas => {
            frame.render_widget(brain::brain_canvas(data, selected), chunks[2]);
        }
        MainView::CostCenter => {
            frame.render_widget(cost::cost_center(data, selected), chunks[2]);
        }
        MainView::EventStream => {
            frame.render_widget(events::event_stream(data, selected), chunks[2]);
        }
        MainView::WorkspaceView => {
            let p = Paragraph::new("Workspace View — coming soon")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Workspace View"),
                )
                .style(Style::default().fg(Color::from_u32(widgets::MUTED_U32)));
            frame.render_widget(p, chunks[2]);
        }
        MainView::Deliverables => {
            let p = Paragraph::new("Deliverables — coming soon")
                .block(Block::default().borders(Borders::ALL).title("Deliverables"))
                .style(Style::default().fg(Color::from_u32(widgets::MUTED_U32)));
            frame.render_widget(p, chunks[2]);
        }
    }

    let footer = Paragraph::new(
        " [1]Kanban [2]Pipeline [3]Mesh [4]Agents [5]Brain [6]Cost [7]Events [8]Workspace [9]Deliverables  [Tab]Next  [q]Quit ",
    )
    .block(Block::default().borders(Borders::ALL))
    .style(Style::default().fg(Color::from_u32(widgets::MUTED_U32)));
    frame.render_widget(footer, chunks[3]);
}

fn view_name(view: MainView) -> &'static str {
    match view {
        MainView::PlanKanban => "Plan Kanban",
        MainView::TaskPipeline => "Task Pipeline",
        MainView::MeshStatus => "Mesh Status",
        MainView::AgentOrgChart => "Agent Org Chart",
        MainView::BrainCanvas => "Brain Canvas",
        MainView::CostCenter => "Cost Center",
        MainView::EventStream => "Event Stream",
        MainView::WorkspaceView => "Workspace View",
        MainView::Deliverables => "Deliverables",
    }
}
