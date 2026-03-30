// Audit middleware: records POST/PUT/DELETE mutations to audit_log after response.
// Runs after response — never blocks the request path.
// Errors are logged via tracing::warn and silently dropped.

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Method, Request};
use axum::middleware::Next;
use axum::response::Response;
use std::net::SocketAddr;

use crate::server::state::ServerState;

/// Extract the agent name from the Authorization header.
/// Returns "dev-mode" or "legacy" for non-JWT tokens, the JWT `sub` claim for JWT tokens.
fn extract_agent(req: &Request<Body>) -> String {
    let header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    let Some(value) = header else {
        return "dev-mode".to_string();
    };

    let Some(token) = value.strip_prefix("Bearer ") else {
        return "legacy".to_string();
    };

    // JWT tokens contain exactly 2 dots
    if token.matches('.').count() == 2 {
        match crate::security::jwt::validate_token(token) {
            Ok(claims) => claims.sub,
            Err(_) => "unknown".to_string(),
        }
    } else {
        "legacy".to_string()
    }
}

/// Extract IP address: try ConnectInfo extension first, then X-Forwarded-For header.
fn extract_ip(req: &Request<Body>) -> Option<String> {
    if let Some(addr) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
        return Some(addr.0.ip().to_string());
    }
    req.headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(',').next().unwrap_or(v).trim().to_string())
}

/// Returns true for mutation methods that should be audited.
fn is_mutation(method: &Method) -> bool {
    matches!(*method, Method::POST | Method::PUT | Method::DELETE)
}

/// Axum middleware that appends a row to `audit_log` for successful (2xx)
/// POST, PUT, and DELETE requests. Never blocks the response.
pub async fn audit_mutations(
    axum::extract::State(state): axum::extract::State<ServerState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let resource = req.uri().path().to_string();
    let agent = extract_agent(&req);
    let ip = extract_ip(&req);

    let resp = next.run(req).await;

    if is_mutation(&method) && resp.status().is_success() {
        let action = method.as_str().to_string();
        let detail = resp.status().as_u16().to_string();

        match state.get_conn() {
            Ok(conn) => {
                if let Err(e) = conn.execute(
                    "INSERT INTO audit_log (agent, action, resource, detail, ip_addr) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![agent, action, resource, detail, ip],
                ) {
                    tracing::warn!(
                        agent = %agent,
                        action = %action,
                        resource = %resource,
                        "audit_log insert failed: {e}"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    agent = %agent,
                    action = %action,
                    resource = %resource,
                    "audit middleware: pool exhausted: {e}"
                );
            }
        }
    }

    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;

    #[test]
    fn test_is_mutation_post() {
        assert!(is_mutation(&Method::POST));
    }

    #[test]
    fn test_is_mutation_put() {
        assert!(is_mutation(&Method::PUT));
    }

    #[test]
    fn test_is_mutation_delete() {
        assert!(is_mutation(&Method::DELETE));
    }

    #[test]
    fn test_is_mutation_get_excluded() {
        assert!(!is_mutation(&Method::GET));
    }

    #[test]
    fn test_extract_agent_no_header() {
        let req = Request::builder()
            .uri("/api/test")
            .body(Body::empty())
            .unwrap();
        assert_eq!(extract_agent(&req), "dev-mode");
    }

    #[test]
    fn test_extract_agent_legacy_bearer() {
        let req = Request::builder()
            .uri("/api/test")
            .header("authorization", "Bearer legacy-token-no-dots")
            .body(Body::empty())
            .unwrap();
        assert_eq!(extract_agent(&req), "legacy");
    }

    #[test]
    fn test_extract_ip_forwarded_header() {
        let req = Request::builder()
            .uri("/api/test")
            .header("x-forwarded-for", "203.0.113.1, 10.0.0.1")
            .body(Body::empty())
            .unwrap();
        assert_eq!(extract_ip(&req), Some("203.0.113.1".to_string()));
    }

    #[test]
    fn test_extract_ip_none() {
        let req = Request::builder()
            .uri("/api/test")
            .body(Body::empty())
            .unwrap();
        assert_eq!(extract_ip(&req), None);
    }
}
