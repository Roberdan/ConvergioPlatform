use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use constant_time_eq::constant_time_eq;
use std::env;
use std::sync::OnceLock;
use tower_http::cors::CorsLayer;

pub fn cors_layer() -> CorsLayer {
    let origins = match env::var("CONVERGIO_CORS_ORIGINS") {
        Ok(value) => {
            let parsed: Vec<_> = value
                .split(',')
                .map(str::trim)
                .filter(|origin| !origin.is_empty())
                .filter_map(|origin| match axum::http::HeaderValue::from_str(origin) {
                    Ok(hv) => Some(hv),
                    Err(e) => { tracing::warn!("invalid CORS origin '{origin}': {e}"); None }
                })
                .collect();
            if parsed.is_empty() { None } else { Some(parsed) }
        }
        Err(_) => None,
    }
        .unwrap_or_else(|| {
            vec![
                axum::http::HeaderValue::from_static("http://localhost:8420"),
                axum::http::HeaderValue::from_static("http://127.0.0.1:8420"),
                axum::http::HeaderValue::from_static("http://localhost:3000"),
                axum::http::HeaderValue::from_static("tauri://localhost"),
            ]
        });

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
        ])
}

static AUTH_TOKEN: OnceLock<Option<String>> = OnceLock::new();

/// Global dev-mode flag — set once at startup via `set_dev_mode(true)`.
/// When true and no auth token is configured, auth is skipped and binding is
/// restricted to 127.0.0.1 so the server is never reachable from the network.
pub static DEV_MODE: OnceLock<bool> = OnceLock::new();

/// Returns whether dev mode is active (set once at daemon startup).
pub fn is_dev_mode() -> bool {
    *DEV_MODE.get_or_init(|| false)
}

/// Initialise dev-mode flag. Must be called before any request is handled.
/// Subsequent calls are silently ignored (OnceLock semantics).
pub fn set_dev_mode(enabled: bool) {
    let _ = DEV_MODE.set(enabled);
    if enabled {
        tracing::warn!("Auth disabled — dev mode, binding to localhost only");
    }
}

fn get_auth_token() -> &'static Option<String> {
    AUTH_TOKEN.get_or_init(|| match env::var("CONVERGIO_AUTH_TOKEN") {
        Ok(t) if !t.is_empty() => Some(t),
        _ => None,
    })
}

/// Compares two string tokens in constant time to prevent timing attacks.
/// Uses byte-level comparison; returns true only when both slices are identical.
pub(crate) fn compare_tokens(a: &str, b: &str) -> bool {
    constant_time_eq(a.as_bytes(), b.as_bytes())
}

/// Returns true when the provided Bearer token matches the configured secret.
///
/// Behaviour:
/// - Token configured → validate header; reject on mismatch or missing header.
/// - No token configured + dev-mode → allow all (localhost-only binding enforced separately).
/// - No token configured + production → deny all (fail-secure default).
pub fn check_bearer(header_value: Option<&str>) -> bool {
    match get_auth_token() {
        None => {
            // No token set: allow only in explicit dev-mode; deny in production.
            is_dev_mode()
        }
        Some(expected) => header_value
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|t| compare_tokens(t, expected.as_str()))
            .unwrap_or(false),
    }
}

/// Routes that never require a Bearer token regardless of auth configuration.
const EXEMPT_ROUTES: &[&str] = &["/api/health"];

/// Returns true if this request requires Bearer auth.
/// All routes require auth except those listed in EXEMPT_ROUTES.
/// Dev-mode bypasses auth entirely (handled in `check_bearer`).
fn needs_auth(_method: &Method, path: &str) -> bool {
    !EXEMPT_ROUTES.contains(&path)
}

/// Axum middleware: rejects requests without a valid Bearer token.
/// Only /api/health is exempt. Auth disabled in dev-mode (localhost binding enforced).
pub async fn require_auth(req: Request<Body>, next: Next) -> Response {
    if !needs_auth(req.method(), req.uri().path()) {
        return next.run(req).await;
    }

    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| match v.to_str() {
            Ok(s) => Some(s),
            Err(e) => { tracing::debug!("auth header not valid UTF-8: {e}"); None }
        });

    if check_bearer(auth_header) {
        next.run(req).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized",
                "message": "Valid Bearer token required"
            })),
        )
            .into_response()
    }
}

/// Middleware that ensures responses include a Cache-Control header when absent.
/// Simple default: private, max-age=10
pub async fn set_cache_headers(req: Request<Body>, next: Next) -> Response {
    use axum::http::header::CACHE_CONTROL;
    use axum::http::HeaderValue;

    let mut res = next.run(req).await;
    if !res.headers().contains_key(CACHE_CONTROL) {
        res.headers_mut().insert(
            CACHE_CONTROL,
            HeaderValue::from_static("private, max-age=10"),
        );
    }
    res
}

#[cfg(test)]
#[path = "middleware_tests.rs"]
mod tests;
