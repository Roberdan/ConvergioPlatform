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
use super::api;

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
    pub(crate) terminal: Terminal<CrosstermBackend<io::Stdout>>,
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

    /// Test constructor — no raw mode, fixed localhost URL.
    #[cfg(test)]
    pub fn new_for_test(view: MainView) -> io::Result<Self> {
        let api_url = "http://localhost:8420".to_string();
        Ok(Self {
            data: TuiData::default(),
            active_view: view,
            selected_index: 0,
            last_fetch: Instant::now() - Duration::from_secs(10),
            ws_client: WsClient::new(&api_url),
            istate: InteractiveState::default(),
            auto_refresh: false,
            refresh_interval_secs: Self::default_refresh_interval_secs(),
            chat: ChatState::default(),
            terminal: Terminal::new(CrosstermBackend::new(io::stdout()))?,
            client: Client::new(),
            api_url,
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
                    // Poll streaming chat events every render tick (100ms).
                    self.poll_chat_events();
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
            if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
                return true;
            }
            // View-switch keys (0-9, Tab) pass through.
            let is_view_switch = matches!(
                code,
                KeyCode::Char('0'..='9') | KeyCode::Tab | KeyCode::BackTab
            );
            if !is_view_switch {
                // Enter triggers chat send via handle_enter -> process_post_key.
                if code == KeyCode::Enter {
                    self.handle_enter();
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
            KeyCode::Char(n @ '0'..='9') => self.switch_view(n as u8 - b'0'),
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
            KeyCode::Enter => self.handle_enter(),
            _ => {
                if input::handle_key(code, modifiers, &mut self.istate) {
                    return true;
                }
            }
        }
        false
    }

    fn switch_view(&mut self, n: u8) {
        use MainView::*;
        self.selected_index = 0;
        // 0 = Deliverables (10th), 1-9 = ordered views
        let views = [Deliverables, PlanKanban, Chat, TaskPipeline, MeshStatus,
                     AgentOrgChart, BrainCanvas, CostCenter, EventStream, WorkspaceView];
        self.active_view = views[(n as usize).min(9)];
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

    pub async fn refresh_data(&mut self) {
        api::refresh_all(&self.client, &self.api_url, &mut self.data).await;
        self.last_fetch = Instant::now();
    }

    pub fn http_client(&self) -> &Client { &self.client }
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
