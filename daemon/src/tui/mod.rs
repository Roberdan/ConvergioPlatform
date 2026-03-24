pub mod api;
pub mod app;
mod app_render;
pub mod chat_handler;
pub mod data;
pub mod input;
pub mod refresh;
pub mod render;
pub mod views;
pub mod widgets;
pub mod ws_client;

pub use app::TuiApp;
pub use data::*;

#[cfg(test)]
mod tests;
