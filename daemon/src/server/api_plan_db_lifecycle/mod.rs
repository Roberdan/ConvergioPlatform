pub mod handlers;

#[cfg(test)]
mod tests;

use super::state::ServerState;
use axum::Router;

pub fn router() -> Router<ServerState> {
    Router::new().merge(handlers::router())
}
