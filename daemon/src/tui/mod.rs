pub mod api;
pub mod app;
pub mod data;
pub mod input;
pub mod views;
pub mod widgets;
pub mod ws_client;

pub use app::TuiApp;
pub use data::*;

#[cfg(test)]
mod tests;
