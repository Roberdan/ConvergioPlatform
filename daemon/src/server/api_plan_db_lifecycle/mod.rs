pub mod handlers;
mod lifecycle_validation;

#[cfg(test)]
mod tests;

use super::state::ServerState;
use axum::Router;

pub fn router() -> Router<ServerState> {
    Router::new().merge(handlers::router())
}
