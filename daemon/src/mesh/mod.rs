pub mod auth;
pub mod daemon;
pub mod delegate;
pub mod delegate_monitor;
pub(crate) mod delegate_prompt;
mod delegate_types;
pub mod peer_resolver;
pub mod error;
pub mod handoff;
pub mod http_api;
pub mod intelligence;
pub mod net;
pub mod observability;
pub mod sandbox;
pub mod sync;
mod ws;

// ConvergioMesh modules (merged from ConvergioMesh W3)
pub mod compat;
pub mod coordinator;
pub mod env;
pub mod join;
pub mod network;
pub mod peers;
pub mod profiles;
pub mod qr;
pub mod token;

// Node provisioning / migration modules
pub mod brew;
pub mod macos;
pub mod repos;
pub mod runners;
pub mod shell;
pub mod vscode;
