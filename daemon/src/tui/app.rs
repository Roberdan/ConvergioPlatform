// TuiApp — async event loop, key handling, data refresh.

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers};
use futures_util::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use reqwest::Client;
use tokio::time::interval;

use super::data::{KpiData, MainView, TuiData};
use super::ws_client::WsClient;
use super::{api, views};

pub struct TuiApp {
    pub data: TuiData,
    pub active_view: MainView,
    pub selected_index: usize,
    pub last_fetch: Instant,
    pub api_url: String,
    pub ws_client: WsClient,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    client: Client,
}

impl TuiApp {
    /// Parse --api-url from argv; default http://localhost:8420.
    pub(crate) fn parse_api_url() -> String {
        let args: Vec<String> = std::env::args().collect();
        let pos = args.iter().position(|a| a == "--api-url");
        pos.and_then(|i| args.get(i + 1))
            .cloned()
            .unwrap_or_else(|| "http://localhost:8420".to_string())
    }

    pub fn new() -> io::Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        let api_url = Self::parse_api_url();
        let ws_client = WsClient::new(&api_url);
        Ok(Self {
            data: TuiData::default(),
            active_view: MainView::default(),
            selected_index: 0,
            last_fetch: Instant::now() - Duration::from_secs(10),
            api_url,
            ws_client,
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
                    // HTTP polling is the fallback path when WS has exceeded max retries.
                    // When WS stream is wired in W3/W4, this branch will only run on fallback.
                    if self.ws_client.should_fallback() {
                        self.refresh_data().await;
                    }
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
    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        match code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
            KeyCode::Char('1') => self.active_view = MainView::PlanKanban,
            KeyCode::Char('2') => self.active_view = MainView::TaskPipeline,
            KeyCode::Char('3') => self.active_view = MainView::MeshStatus,
            KeyCode::Char('4') => self.active_view = MainView::AgentOrgChart,
            KeyCode::Char('5') => self.active_view = MainView::BrainCanvas,
            KeyCode::Char('6') => self.active_view = MainView::CostCenter,
            KeyCode::Char('7') => self.active_view = MainView::EventStream,
            KeyCode::Char('8') => self.active_view = MainView::WorkspaceView,
            KeyCode::Char('9') => self.active_view = MainView::Deliverables,
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

    pub fn list_len(&self) -> usize {
        match self.active_view {
            MainView::PlanKanban => self.data.plans.len(),
            MainView::TaskPipeline => self.data.pipeline.len(),
            MainView::MeshStatus => self.data.mesh_nodes.len(),
            MainView::AgentOrgChart => self.data.agents.len(),
            MainView::BrainCanvas => self.data.brain_nodes.len(),
            MainView::CostCenter => self.data.cost.by_model.len(),
            MainView::EventStream => self.data.events.len(),
            MainView::WorkspaceView => self.data.workspaces.len(),
            MainView::Deliverables => self.data.deliverables.len(),
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
        let url = self.api_url.as_str();
        let (kpis, plans, tasks, mesh, agents, (brain_nodes, brain_kpi), cost, summary, events) =
            tokio::join!(
                api::fetch_overview(&self.client, url),
                api::fetch_plans(&self.client, url),
                api::fetch_all_tasks(&self.client, url),
                api::fetch_mesh(&self.client, url),
                api::fetch_agents(&self.client, url),
                api::fetch_brain(&self.client, url),
                api::fetch_cost(&self.client, url),
                api::fetch_metrics_summary(&self.client, url),
                api::fetch_events(&self.client, url),
            );
        // Merge KPIs: overview wins for all fields except tokens/cost if brain provides them
        self.data.kpis = if brain_kpi.daily_tokens > 0 || brain_kpi.daily_cost > 0.0 {
            KpiData {
                daily_tokens: brain_kpi.daily_tokens,
                daily_cost: brain_kpi.daily_cost,
                ..kpis
            }
        } else {
            kpis
        };
        self.data.plans = plans;
        self.data.pipeline = tasks;
        self.data.mesh_nodes = mesh;
        self.data.agents = agents;
        self.data.brain_nodes = brain_nodes;
        self.data.events = events;
        // Merge cost data: combine fetched cost fields with summary
        self.data.cost = crate::tui::CostData {
            by_model: cost.by_model,
            by_project: cost.by_project,
            by_date: cost.by_date,
            summary,
        };
        self.last_fetch = Instant::now();
    }

    pub fn next_view(&mut self) {
        self.selected_index = 0;
        self.active_view = match self.active_view {
            MainView::PlanKanban => MainView::TaskPipeline,
            MainView::TaskPipeline => MainView::MeshStatus,
            MainView::MeshStatus => MainView::AgentOrgChart,
            MainView::AgentOrgChart => MainView::BrainCanvas,
            MainView::BrainCanvas => MainView::CostCenter,
            MainView::CostCenter => MainView::EventStream,
            MainView::EventStream => MainView::WorkspaceView,
            MainView::WorkspaceView => MainView::Deliverables,
            MainView::Deliverables => MainView::PlanKanban,
        };
    }

    pub fn prev_view(&mut self) {
        self.selected_index = 0;
        self.active_view = match self.active_view {
            MainView::PlanKanban => MainView::Deliverables,
            MainView::TaskPipeline => MainView::PlanKanban,
            MainView::MeshStatus => MainView::TaskPipeline,
            MainView::AgentOrgChart => MainView::MeshStatus,
            MainView::BrainCanvas => MainView::AgentOrgChart,
            MainView::CostCenter => MainView::BrainCanvas,
            MainView::EventStream => MainView::CostCenter,
            MainView::WorkspaceView => MainView::EventStream,
            MainView::Deliverables => MainView::WorkspaceView,
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
