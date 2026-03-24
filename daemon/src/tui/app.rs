// TuiApp — async event loop, key handling, data refresh.

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers};
use futures_util::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use reqwest::Client;
use tokio::time::interval;

use super::chat_handler::{self, ChatState};
use super::data::{MainView, TuiData};
use super::input::{self, InteractiveState};
use super::ws_client::WsClient;
use super::{api, views};

pub struct TuiApp {
    pub data: TuiData,
    pub active_view: MainView,
    pub selected_index: usize,
    pub last_fetch: Instant,
    pub api_url: String,
    pub ws_client: WsClient,
    pub istate: InteractiveState,
    /// Whether automatic polling refresh is enabled.
    pub auto_refresh: bool,
    /// Polling interval in seconds (one of: 3, 5, 10, 30, 60).
    pub refresh_interval_secs: u64,
    /// Chat view mutable state.
    pub chat: ChatState,
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
            istate: InteractiveState::default(),
            auto_refresh: Self::default_auto_refresh(),
            refresh_interval_secs: Self::default_refresh_interval_secs(),
            chat: ChatState::default(),
            terminal,
            client: Client::new(),
        })
    }

    /// Main async event loop using tokio::select! on three channels.
    pub async fn run(&mut self) -> io::Result<()> {
        let mut events = EventStream::new();
        // poll_tick is re-created whenever the interval changes.
        let mut poll_tick = interval(Duration::from_secs(self.refresh_interval_secs));
        let mut render_tick = interval(Duration::from_millis(100));

        // Initial data fetch before first render
        self.refresh_data().await;

        loop {
            tokio::select! {
                _ = poll_tick.tick() => {
                    // HTTP polling fallback when WS has exceeded max retries.
                    // Only poll when auto_refresh is enabled.
                    if self.auto_refresh && self.ws_client.should_fallback() {
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
                            // Handle force-refresh, interval changes, and command-enter.
                            let interval_changed = self.process_post_key().await;
                            if interval_changed {
                                // Re-create poll_tick with the new interval.
                                poll_tick = interval(Duration::from_secs(self.refresh_interval_secs));
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

    /// Dispatch key to input module; also handle view-switch and nav keys.
    /// Returns true if the app should quit.
    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        // Command mode and help overlay handled by input::handle_key.
        if self.istate.command_mode || self.istate.show_help {
            let was_mode = self.istate.command_mode;
            let quit = input::handle_key(code, modifiers, &mut self.istate);
            if was_mode && !self.istate.command_mode && !self.istate.command_input.is_empty() {
                let cmd = std::mem::take(&mut self.istate.command_input);
                input::parse_and_apply_command(&cmd, &mut self.istate, &mut self.active_view);
            }
            return quit || self.istate.quit;
        }

        // Chat view captures most keys for input composition.
        if self.active_view == MainView::Chat && !self.chat.sending {
            // Allow Ctrl-C and view-switch keys to pass through.
            if modifiers.contains(KeyModifiers::CONTROL) {
                if code == KeyCode::Char('c') { return true; }
            }
            // View-switch keys (0-9, Tab) pass through.
            let is_view_switch = matches!(code,
                KeyCode::Char('0'..='9') | KeyCode::Tab | KeyCode::BackTab);
            if !is_view_switch {
                // Enter triggers send (handled async in process_post_key).
                if code == KeyCode::Enter {
                    self.istate.force_refresh = false; // sentinel: chat send pending
                    return false;
                }
                if chat_handler::handle_chat_key(code, &mut self.chat) {
                    return false;
                }
            }
        }

        // Normal key handling.
        match code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
            KeyCode::Char('0') => {
                self.selected_index = 0;
                self.active_view = MainView::Chat;
            }
            KeyCode::Char(n @ '1'..='9') => self.switch_view(n as u8 - b'0'),
            KeyCode::Tab => self.next_view(),
            KeyCode::BackTab => self.prev_view(),
            KeyCode::Up => {
                self.selected_index = self.selected_index.saturating_sub(1);
            }
            KeyCode::Down => {
                let max = self.list_len().saturating_sub(1);
                if self.selected_index < max { self.selected_index += 1; }
            }
            KeyCode::Enter => self.handle_enter(),
            _ => {
                if input::handle_key(code, modifiers, &mut self.istate) { return true; }
            }
        }
        false
    }

    fn switch_view(&mut self, n: u8) {
        self.selected_index = 0;
        self.active_view = [
            MainView::PlanKanban, MainView::TaskPipeline, MainView::MeshStatus,
            MainView::AgentOrgChart, MainView::BrainCanvas, MainView::CostCenter,
            MainView::EventStream, MainView::WorkspaceView, MainView::Deliverables,
        ][(n as usize).saturating_sub(1).min(8)];
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
            MainView::Chat => self.data.chat_messages.len(),
        }
    }

    fn render(&mut self) -> io::Result<()> {
        let view = self.active_view;
        let data = &self.data;
        let selected = self.selected_index;
        let api_url = self.api_url.clone();
        let show_help = self.istate.show_help;
        let auto_refresh = self.auto_refresh;
        let refresh_interval_secs = self.refresh_interval_secs;
        let cmd_input = self.istate.command_mode.then(|| self.istate.command_input.clone());
        let detail = self.istate.detail_text.clone();
        let chat_input = self.chat.input.clone();
        let chat_sending = self.chat.sending;
        self.terminal.draw(|frame| {
            views::render_view(
                frame, frame.area(), view, data, selected, &api_url,
                show_help, auto_refresh, refresh_interval_secs,
                &chat_input, chat_sending,
            );
            if let Some(d) = &detail {
                views::render_detail_popup(frame, frame.area(), d);
            }
            // Overlay command footer at bottom whenever in command mode.
            if cmd_input.is_some() {
                let area = frame.area();
                let fh = 3_u16.min(area.height);
                let footer = ratatui::layout::Rect {
                    x: area.x, y: area.y + area.height.saturating_sub(fh),
                    width: area.width, height: fh,
                };
                views::render_command_footer(frame, footer, cmd_input.as_deref());
            }
        })?;
        Ok(())
    }

    pub async fn refresh_data(&mut self) {
        api::refresh_all(&self.client, &self.api_url, &mut self.data).await;
        self.last_fetch = Instant::now();
    }

    /// Exposes the HTTP client for modules that impl on TuiApp (e.g. refresh.rs).
    pub fn http_client(&self) -> &Client {
        &self.client
    }

    pub fn next_view(&mut self) {
        self.selected_index = 0;
        // 10 views total (0-indexed 0..9); Tab cycles through all.
        let idx = Self::view_index(self.active_view);
        self.active_view = Self::view_at((idx + 1) % 10);
    }

    pub fn prev_view(&mut self) {
        self.selected_index = 0;
        let idx = Self::view_index(self.active_view);
        self.active_view = Self::view_at((idx + 9) % 10);
    }

    fn view_index(v: MainView) -> usize {
        use MainView::*;
        [PlanKanban, TaskPipeline, MeshStatus, AgentOrgChart, BrainCanvas,
         CostCenter, EventStream, WorkspaceView, Deliverables, Chat]
            .iter().position(|x| *x == v).unwrap_or(0)
    }

    fn view_at(idx: usize) -> MainView {
        use MainView::*;
        [PlanKanban, TaskPipeline, MeshStatus, AgentOrgChart, BrainCanvas,
         CostCenter, EventStream, WorkspaceView, Deliverables, Chat]
            .get(idx).copied().unwrap_or(PlanKanban)
    }
}

impl Drop for TuiApp {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture);
    }
}
