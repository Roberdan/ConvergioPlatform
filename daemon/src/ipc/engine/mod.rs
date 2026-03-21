mod agents;
mod channels_context;
mod dispatch;
mod messaging;

pub mod core;

pub use core::{IpcEngine, DEFAULT_RATE_LIMIT};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_ctx_dispatch;
