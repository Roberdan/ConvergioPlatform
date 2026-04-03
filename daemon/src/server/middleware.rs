use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use constant_time_eq::constant_time_eq;
use std::env;
use std::sync::OnceLock;
use tower_http::cors::CorsLayer;

use crate::security::jwt::{self, AgentClaims};
use crate::security::rbac;
use crate::server::middleware_mesh::{verify_body_hash, verify_mesh_hmac};

pub fn cors_layer() -> CorsLayer {
    let origins = env::var("CONVERGIO_CORS_ORIGINS").ok().and_then(|value| {
        let parsed: Vec<_> = value.split(',').map(str::trim).filter(|o| !o.is_empty())
            .filter_map(|o| axum::http::HeaderValue::from_str(o).ok()).collect();
        if parsed.is_empty() { None } else { Some(parsed) }
    }).unwrap_or_else(|| vec![
        axum::http::HeaderValue::from_static("http://localhost:8420"),
        axum::http::HeaderValue::from_static("http://127.0.0.1:8420"),
        axum::http::HeaderValue::from_static("http://localhost:3000"),
        axum::http::HeaderValue::from_static("tauri://localhost"),
    ]);

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
/// When true **and** the `--dev-mode` CLI flag was explicitly passed,
/// auth is skipped and binding is restricted to 127.0.0.1.
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
        tracing::warn!(
            "Dev mode active (--dev-mode flag) — auth disabled, localhost only"
        );
    }
}

fn get_auth_token() -> &'static Option<String> {
    AUTH_TOKEN.get_or_init(|| match env::var("CONVERGIO_AUTH_TOKEN") {
        Ok(t) if !t.is_empty() => Some(t),
        _ => None,
    })
}

/// Compares two string tokens in constant time to prevent timing attacks.
pub(crate) fn compare_tokens(a: &str, b: &str) -> bool {
    constant_time_eq(a.as_bytes(), b.as_bytes())
}

/// Authenticate a request. Returns Ok(Some(claims)) for JWT,
/// Ok(None) for legacy bearer, mesh HMAC, or dev-mode, Err for denied.
fn authenticate(
    header_value: Option<&str>,
    mesh_headers: Option<(&str, &str)>,
    path_and_query: &str,
    method: &str,
    body_hash: Option<&str>,
) -> Result<Option<AgentClaims>, ()> {
    // 0. Mesh HMAC auth for sync endpoints — peers use shared_secret
    if let Some((timestamp, signature)) = mesh_headers {
        if path_and_query.starts_with("/api/sync/") || path_and_query.starts_with("/api/mesh/") {
            return verify_mesh_hmac(timestamp, signature, path_and_query, method, body_hash);
        }
    }

    // 1. Try JWT first (Bearer <jwt-with-dots>)
    if let Some(token) = header_value
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        // JWT tokens have 2 dots; legacy tokens do not
        if token.matches('.').count() == 2 {
            return match jwt::validate_token(token) {
                Ok(claims) => Ok(Some(claims)),
                Err(e) => {
                    tracing::warn!("JWT validation failed: {e}");
                    Err(())
                }
            };
        }
        // Legacy shared bearer token — assign system role for RBAC
        if let Some(expected) = get_auth_token() {
            if compare_tokens(token, expected.as_str()) {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                return Ok(Some(AgentClaims {
                    sub: "system-bearer".to_string(),
                    role: jwt::AgentRole::Coordinator,
                    cap: vec!["*".to_string()],
                    iat: now,
                    exp: u64::MAX,
                }));
            }
        }
        return Err(());
    }

    // 2. No Authorization header
    match get_auth_token() {
        None if is_dev_mode() => Ok(None),
        _ => Err(()),
    }
}

/// Routes that never require a Bearer token regardless of auth config.
const EXEMPT_ROUTES: &[&str] = &["/api/health"];

/// Returns true if this request requires auth.
fn needs_auth(_method: &Method, path: &str) -> bool {
    !EXEMPT_ROUTES.contains(&path)
}

/// Returns true if the request originates from localhost (127.0.0.1, ::1, or unix socket).
fn is_localhost(req: &Request<Body>) -> bool {
    use axum::extract::ConnectInfo;
    // Check x-forwarded-for (if behind reverse proxy, don't trust)
    if req.headers().contains_key("x-forwarded-for") {
        return false;
    }
    // Check peer address from connection info extension
    if let Some(addr) = req.extensions().get::<ConnectInfo<std::net::SocketAddr>>() {
        return addr.0.ip().is_loopback();
    }
    // No ConnectInfo available — cannot determine origin. Refuse to trust.
    false
}

/// Axum middleware: authenticates via JWT (with RBAC), legacy bearer, or mesh HMAC.
/// /api/health is exempt. Localhost requests skip auth (scripts/CLI on same machine).
/// Dev-mode (--dev-mode flag) also bypasses auth.
pub async fn require_auth(req: Request<Body>, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let path_and_query = req.uri().path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| path.clone());
    let method_str = req.method().to_string();

    if !needs_auth(req.method(), &path) {
        return next.run(req).await;
    }

    // Localhost bypass: trust requests from 127.0.0.1 / ::1 / unix socket.
    if is_localhost(&req) {
        tracing::debug!(path = %path, "localhost auth bypass");
        return next.run(req).await;
    }

    let auth_header = req.headers().get("authorization")
        .and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    let mesh_ts = req.headers().get("x-mesh-timestamp")
        .and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    let mesh_sig = req.headers().get("x-mesh-signature")
        .and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    let mesh_body_hash = req.headers().get("x-mesh-body-hash")
        .and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    let mesh_headers = mesh_ts.as_deref().zip(mesh_sig.as_deref());

    // For mesh-auth POST with body hash: buffer body, verify hash integrity
    let req = if mesh_headers.is_some() && mesh_body_hash.is_some() && req.method() == Method::POST {
        let (parts, body) = req.into_parts();
        let bytes = match axum::body::to_bytes(body, 10_485_760).await {
            Ok(b) => b,
            Err(_) => return (StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error":"Body too large"}))).into_response(),
        };
        if !verify_body_hash(&bytes, mesh_body_hash.as_ref().unwrap()) {
            return (StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error":"Body hash mismatch"}))).into_response();
        }
        Request::from_parts(parts, Body::from(bytes))
    } else {
        req
    };

    match authenticate(auth_header.as_deref(), mesh_headers, &path_and_query, &method_str, mesh_body_hash.as_deref()) {
        Ok(Some(claims)) => {
            if !rbac::role_can_access(&claims.role, &path) {
                tracing::warn!(agent=%claims.sub, role=%claims.role, path=%path, "RBAC denied");
                return (StatusCode::FORBIDDEN, Json(serde_json::json!({
                    "error": "Forbidden",
                    "message": format!("Role '{}' cannot access {path}", claims.role)
                }))).into_response();
            }
            tracing::debug!(agent=%claims.sub, role=%claims.role, path=%path, "Authenticated request");
            next.run(req).await
        }
        Ok(None) => {
            tracing::debug!(path = %path, "Mesh HMAC or dev-mode auth");
            next.run(req).await
        }
        Err(()) => (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "error": "Unauthorized", "message": "Valid Bearer token required"
        }))).into_response(),
    }
}

/// Middleware that ensures responses include a Cache-Control header.
pub async fn set_cache_headers(req: Request<Body>, next: Next) -> Response {
    let mut res = next.run(req).await;
    if !res.headers().contains_key(axum::http::header::CACHE_CONTROL) {
        res.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("private, max-age=10"),
        );
    }
    res
}

#[cfg(test)]
#[path = "middleware_tests.rs"]
mod tests;
