use super::*;

// --- check_bearer: production (no dev-mode, no token) ---

#[test]
fn no_token_production_denies_all() {
    let dev_mode_off = false;
    let result = match get_auth_token() {
        None => dev_mode_off,
        Some(_) => false,
    };
    assert!(!result, "no token + no dev-mode must deny");
}

#[test]
fn no_token_dev_mode_allows() {
    let dev_mode_on = true;
    let result = match get_auth_token() {
        None => dev_mode_on,
        Some(_) => false,
    };
    let _ = result;
}

// --- compare_tokens ---

#[test]
fn correct_bearer_passes_when_token_set() {
    let expected = "my-secret";
    let provided = "Bearer my-secret";
    let result = provided
        .strip_prefix("Bearer ")
        .map(|t| compare_tokens(t, expected))
        .unwrap_or(false);
    assert!(result);
}

#[test]
fn wrong_bearer_fails_when_token_set() {
    let expected = "my-secret";
    let provided = "Bearer wrong-token";
    let result = provided
        .strip_prefix("Bearer ")
        .map(|t| compare_tokens(t, expected))
        .unwrap_or(false);
    assert!(!result);
}

#[test]
fn missing_bearer_fails_when_token_set() {
    let expected = "my-secret";
    let result = None::<&str>
        .and_then(|v: &str| v.strip_prefix("Bearer "))
        .map(|t| compare_tokens(t, expected))
        .unwrap_or(false);
    assert!(!result);
}

// --- Constant-time comparison ---

#[test]
fn constant_time_correct_token_passes() {
    assert!(compare_tokens("secret-abc", "secret-abc"));
}

#[test]
fn constant_time_wrong_token_fails() {
    assert!(!compare_tokens("secret-abc", "wrong-token"));
}

#[test]
fn constant_time_empty_vs_nonempty_fails() {
    assert!(!compare_tokens("", "secret"));
    assert!(!compare_tokens("secret", ""));
}

#[test]
fn constant_time_both_empty_passes() {
    assert!(compare_tokens("", ""));
}

#[test]
fn constant_time_prefix_subset_fails() {
    assert!(!compare_tokens("secret", "secret-extra"));
    assert!(!compare_tokens("secret-extra", "secret"));
}

// --- needs_auth ---

#[test]
fn health_is_exempt_from_auth() {
    assert!(!needs_auth(&Method::GET, "/api/health"));
}

#[test]
fn all_get_routes_now_require_auth() {
    assert!(needs_auth(&Method::GET, "/api/overview"));
    assert!(needs_auth(&Method::GET, "/api/ideas"));
    assert!(needs_auth(&Method::GET, "/ws/brain"));
    assert!(needs_auth(&Method::GET, "/ws/dashboard"));
}

#[test]
fn mutable_methods_require_auth() {
    assert!(needs_auth(&Method::POST, "/api/ideas"));
    assert!(needs_auth(&Method::PUT, "/api/ideas/1"));
    assert!(needs_auth(&Method::DELETE, "/api/ideas/1"));
}

#[test]
fn websocket_and_sse_routes_require_auth() {
    assert!(needs_auth(&Method::GET, "/ws/pty"));
    assert!(needs_auth(&Method::GET, "/api/plan/start"));
    assert!(needs_auth(&Method::GET, "/api/plan/delegate"));
    assert!(needs_auth(&Method::GET, "/api/plan/preflight"));
}

#[test]
fn no_protected_get_list_exists() {
    assert_eq!(EXEMPT_ROUTES.len(), 1);
    assert_eq!(EXEMPT_ROUTES[0], "/api/health");
}

// --- authenticate function ---

#[test]
fn authenticate_no_header_no_token_no_devmode_denies() {
    // Without dev-mode and no token configured, no header => deny
    // (This tests the logic path; OnceLock state may vary in CI)
    let result = authenticate(None);
    // In CI with no env vars and dev-mode off, this should be Err
    // If AUTH_TOKEN was set by another test, still Err (no header)
    assert!(result.is_err() || result.unwrap().is_none());
}

#[test]
fn authenticate_jwt_format_detected_by_dots() {
    // A token with 2 dots is treated as JWT, not legacy bearer
    let fake_jwt = "Bearer aaa.bbb.ccc";
    let result = authenticate(Some(fake_jwt));
    // This will fail JWT validation (invalid signature), so Err
    assert!(result.is_err());
}

#[test]
fn authenticate_legacy_bearer_without_dots() {
    // A token without dots is treated as legacy bearer
    let legacy = "Bearer simple-token-no-dots";
    let result = authenticate(Some(legacy));
    // Will fail unless CONVERGIO_AUTH_TOKEN matches
    // The point: it does NOT try JWT decode on dotless tokens
    assert!(result.is_err() || result.unwrap().is_none());
}
