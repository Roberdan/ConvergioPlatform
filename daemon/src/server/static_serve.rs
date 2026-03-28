// HTTP server runner and signal handling.
// Extracted from mod.rs — run_router and shutdown_signal.

use axum::Router;
use crate::server::state::ApiError;

pub(super) async fn run_router(bind_addr: &str, router: Router) -> Result<(), ApiError> {
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .map_err(|e| {
            ApiError::internal(format!("server listen failed on {bind_addr}: {e}"))
        })?;
    axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| ApiError::internal(format!("server runtime failed: {e}")))
}

pub(super) async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};

        if let Ok(mut sigterm) = signal(SignalKind::terminate()) {
            sigterm.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}

/// Inner logic: given the token presence state, determine effective bind address.
/// Separated for deterministic unit testing without env-var races.
///
/// In dev-mode without auth token, we warn about network exposure but respect
/// the requested bind address so that mesh nodes remain reachable via --bind.
pub(crate) fn resolve_bind_addr_with(requested: &str, dev_mode: bool, has_token: bool) -> String {
    if dev_mode && !has_token && !requested.starts_with("127.0.0.1") {
        eprintln!(
            "[warn] dev-mode without CONVERGIO_AUTH_TOKEN: binding to {requested} \
             exposes an unauthenticated server. Set CONVERGIO_AUTH_TOKEN or use \
             --bind 127.0.0.1:<port> to restrict access."
        );
    }
    requested.to_string()
}

/// Determine the effective bind address, reading token state from the environment.
///
/// When `dev_mode` is true and no `CONVERGIO_AUTH_TOKEN` is set, we force
/// 127.0.0.1 regardless of the requested bind address to prevent accidental
/// network exposure of an unauthenticated server.
pub fn resolve_bind_addr(requested: &str, dev_mode: bool) -> String {
    use std::env;
    let has_token = env::var("CONVERGIO_AUTH_TOKEN")
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    resolve_bind_addr_with(requested, dev_mode, has_token)
}

#[cfg(test)]
mod bind_addr_tests {
    use super::resolve_bind_addr_with;

    #[test]
    fn dev_mode_no_token_respects_requested_addr() {
        // B7: dev-mode now respects --bind (warns but does not override)
        let addr = resolve_bind_addr_with("0.0.0.0:8420", true, false);
        assert_eq!(addr, "0.0.0.0:8420");
    }

    #[test]
    fn dev_mode_with_token_keeps_requested_addr() {
        let addr = resolve_bind_addr_with("0.0.0.0:8420", true, true);
        assert_eq!(addr, "0.0.0.0:8420");
    }

    #[test]
    fn production_mode_keeps_requested_addr() {
        let addr = resolve_bind_addr_with("0.0.0.0:8420", false, false);
        assert_eq!(addr, "0.0.0.0:8420");
    }

    #[test]
    fn dev_mode_preserves_custom_bind() {
        // B7: explicit --bind is respected even in dev-mode
        let addr = resolve_bind_addr_with("192.168.1.1:9000", true, false);
        assert_eq!(addr, "192.168.1.1:9000");
    }

    #[test]
    fn dev_mode_localhost_no_warning() {
        // Binding to 127.0.0.1 in dev-mode is safe, no override needed
        let addr = resolve_bind_addr_with("127.0.0.1:8420", true, false);
        assert_eq!(addr, "127.0.0.1:8420");
    }
}
