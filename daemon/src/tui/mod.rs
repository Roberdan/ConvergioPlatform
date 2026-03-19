pub mod api;
pub mod views;
pub mod widgets;

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers};
use futures_util::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use reqwest::Client;
use tokio::time::interval;

// --- Data structs (preserved from prior implementation) ---

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanCard {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub tasks_done: i64,
    pub tasks_total: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskPipelineItem {
    pub task_id: String,
    pub title: String,
    pub status: String,
    pub agent: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshNode {
    pub name: String,
    pub online: bool,
    pub role: String,
    pub cpu_percent: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentOrgNode {
    pub name: String,
    pub role: String,
    pub host: String,
    pub active_task: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct KpiData {
    pub plans_active: i64,
    pub agents_running: i64,
    pub daily_tokens: i64,
    pub daily_cost: f64,
    pub mesh_online: i64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TuiData {
    pub plans: Vec<PlanCard>,
    pub pipeline: Vec<TaskPipelineItem>,
    pub mesh_nodes: Vec<MeshNode>,
    pub agents: Vec<AgentOrgNode>,
    pub kpis: KpiData,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MainView {
    #[default]
    PlanKanban,
    TaskPipeline,
    MeshStatus,
    AgentOrgChart,
}

// --- TuiApp ---

pub struct TuiApp {
    pub data: TuiData,
    pub active_view: MainView,
    pub selected_index: usize,
    pub last_fetch: Instant,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    client: Client,
}

impl TuiApp {
    pub fn new() -> io::Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            data: TuiData::default(),
            active_view: MainView::default(),
            selected_index: 0,
            last_fetch: Instant::now() - Duration::from_secs(10),
            terminal,
            client: Client::new(),
        })
    }

    /// Main async event loop using tokio::select! on three channels.
    pub async fn run(&mut self) -> io::Result<()> {
        let mut events = EventStream::new();
        let mut poll_tick = interval(Duration::from_secs(5));
        let mut render_tick = interval(Duration::from_millis(100));

        // Initial data fetch before first render
        self.refresh_data().await;

        loop {
            tokio::select! {
                _ = poll_tick.tick() => {
                    self.refresh_data().await;
                }
                _ = render_tick.tick() => {
                    self.render()?;
                }
                maybe_event = events.next() => {
                    match maybe_event {
                        Some(Ok(Event::Key(key))) => {
                            if self.handle_key(key.code, key.modifiers) {
                                return Ok(());
                            }
                        }
                        Some(Err(_)) => return Ok(()),
                        None => return Ok(()),
                        _ => {}
                    }
                }
            }
        }
    }

    /// Returns true if the app should quit.
    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        match code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
            KeyCode::Char('1') => self.active_view = MainView::PlanKanban,
            KeyCode::Char('2') => self.active_view = MainView::TaskPipeline,
            KeyCode::Char('3') => self.active_view = MainView::MeshStatus,
            KeyCode::Char('4') => self.active_view = MainView::AgentOrgChart,
            KeyCode::Tab => self.next_view(),
            KeyCode::BackTab => self.prev_view(),
            KeyCode::Up => {
                self.selected_index = self.selected_index.saturating_sub(1);
            }
            KeyCode::Down => {
                let max = self.list_len().saturating_sub(1);
                if self.selected_index < max {
                    self.selected_index += 1;
                }
            }
            KeyCode::Enter => {} // reserved for future drill-down
            _ => {}
        }
        false
    }

    fn list_len(&self) -> usize {
        match self.active_view {
            MainView::PlanKanban => self.data.plans.len(),
            MainView::TaskPipeline => self.data.pipeline.len(),
            MainView::MeshStatus => self.data.mesh_nodes.len(),
            MainView::AgentOrgChart => self.data.agents.len(),
        }
    }

    fn render(&mut self) -> io::Result<()> {
        let view = self.active_view;
        let data = &self.data;
        let selected = self.selected_index;
        self.terminal.draw(|frame| {
            views::render_view(frame, frame.area(), view, data, selected);
        })?;
        Ok(())
    }

    async fn refresh_data(&mut self) {
        let (kpis, plans, tasks, mesh, agents) = tokio::join!(
            api::fetch_overview(&self.client),
            api::fetch_plans(&self.client),
            api::fetch_all_tasks(&self.client),
            api::fetch_mesh(&self.client),
            api::fetch_agents(&self.client),
        );
        self.data.kpis = kpis;
        self.data.plans = plans;
        self.data.pipeline = tasks;
        self.data.mesh_nodes = mesh;
        self.data.agents = agents;
        self.last_fetch = Instant::now();
    }

    pub fn next_view(&mut self) {
        self.selected_index = 0;
        self.active_view = match self.active_view {
            MainView::PlanKanban => MainView::TaskPipeline,
            MainView::TaskPipeline => MainView::MeshStatus,
            MainView::MeshStatus => MainView::AgentOrgChart,
            MainView::AgentOrgChart => MainView::PlanKanban,
        };
    }

    pub fn prev_view(&mut self) {
        self.selected_index = 0;
        self.active_view = match self.active_view {
            MainView::PlanKanban => MainView::AgentOrgChart,
            MainView::TaskPipeline => MainView::PlanKanban,
            MainView::MeshStatus => MainView::TaskPipeline,
            MainView::AgentOrgChart => MainView::MeshStatus,
        };
    }
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture
        );
    }
}

#[cfg(test)]
mod tests;

