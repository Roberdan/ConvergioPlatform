use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::{MainView, TuiData};
use crate::tui::widgets;

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
    }

    let footer =
        Paragraph::new(" [1] Kanban  [2] Pipeline  [3] Mesh  [4] Org  [Tab] Next  [q] Quit ")
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
    }
}
