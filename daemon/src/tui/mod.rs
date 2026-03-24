pub mod api;
pub mod app;
mod app_render;
pub mod chat_handler;
pub mod chat_messages;
pub mod claude_session;
pub(crate) mod session_lifecycle;
pub mod data;
pub mod drill_down;
pub mod input;
pub mod refresh;
mod tree_nav;
pub mod views;
pub mod widgets;
pub mod ws_client;

pub use app::TuiApp;
pub use data::*;

#[cfg(test)]
mod tests;

/// Test helpers for constructing TuiApp instances in unit tests.
/// Provides make_app_with_view which sets up realistic sample data.
#[cfg(test)]
pub mod refresh_test_helpers {
    use super::app::TuiApp;
    use super::data::MainView;
    use super::tests::sample_data;

    /// Create a TuiApp set to the given view, pre-loaded with sample data from tests::sample_data.
    pub fn make_app_with_view(view: MainView) -> TuiApp {
        let mut app = TuiApp::new_for_test(view).expect("test TuiApp");
        app.data = sample_data();
        app
    }
}
