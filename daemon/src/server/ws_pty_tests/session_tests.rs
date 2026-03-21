//! Tests for PTY session management: live Tailscale resolution, local peer detection,
//! session limits, process spawning, and WebSocket message routing.

use super::test_state;

// ── Tailscale live resolution (requires tailscale running) ──────────

#[test]
fn tailscale_resolve_self() {
    let ts_out = std::process::Command::new("tailscale")
        .arg("status")
        .output();
    match &ts_out {
        Ok(o) if o.status.success() => {}
        _ => return, // skip if tailscale not running
    }
    // Use the local hostname to test self-resolution
    let hostname = std::process::Command::new("hostname")
        .arg("-s")
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .to_lowercase()
                .to_string()
        })
        .unwrap_or_default();
    // Find matching peer name from known peers
    let peer = if hostname.contains("m3") || hostname.contains("roberto") {
        "mac-worker-2"
    } else if hostname.contains("m1") || hostname.contains("mario") {
        "mac-worker-1"
    } else {
        return;
    }; // unknown host, skip
    let result = super::super::ws_pty::tailscale_resolve(peer);
    if let Some((ip, _online, is_self)) = result {
        assert!(ip.starts_with("100."), "expected Tailscale IP, got: {ip}");
        assert!(is_self, "{peer} should be self on this host");
    }
}

#[test]
fn tailscale_resolve_remote() {
    if std::process::Command::new("tailscale")
        .arg("status")
        .output()
        .is_err()
    {
        return;
    }
    let result = super::super::ws_pty::tailscale_resolve("linux-worker");
    if let Some((ip, _online, is_self)) = result {
        assert!(ip.starts_with("100."));
        assert!(!is_self, "linux-worker should not be self");
    }
}

// ── Local peer detection ────────────────────────────────────────────

#[test]
fn is_local_literal_local() {
    let state = test_state();
    assert!(super::super::ws_pty::is_local_peer(&state, "local"));
}

#[test]
fn is_local_literal_localhost() {
    let state = test_state();
    assert!(super::super::ws_pty::is_local_peer(&state, "localhost"));
}

#[test]
fn is_local_self_via_tailscale() {
    let state = test_state();
    let ts_out = std::process::Command::new("tailscale")
        .arg("status")
        .output();
    match &ts_out {
        Ok(o) if o.status.success() => {}
        _ => return, // skip if tailscale not running
    }
    let hostname = std::process::Command::new("hostname")
        .arg("-s")
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .to_lowercase()
                .to_string()
        })
        .unwrap_or_default();
    let peer = if hostname.contains("m3") || hostname.contains("roberto") {
        "mac-worker-2"
    } else if hostname.contains("m1") || hostname.contains("mario") {
        "mac-worker-1"
    } else {
        return;
    };
    assert!(super::super::ws_pty::is_local_peer(&state, peer));
}

#[test]
fn is_remote_via_tailscale() {
    let state = test_state();
    if std::process::Command::new("tailscale")
        .arg("status")
        .output()
        .is_err()
    {
        return;
    }
    assert!(!super::super::ws_pty::is_local_peer(&state, "linux-worker"));
}

// ── Session limits ───────────────────────────────────────────────────

#[test]
fn session_limit_constants() {
    assert_eq!(super::super::ws_pty::MAX_PTY_SESSIONS, 10);
    // ACTIVE_SESSIONS starts at 0 (may be non-zero if other tests ran concurrently)
    let _ = super::super::ws_pty::ACTIVE_SESSIONS.load(std::sync::atomic::Ordering::SeqCst);
}

// ── tokio::process spawn ────────────────────────────────────────────

#[tokio::test]
async fn spawn_echo_produces_output() {
    use tokio::io::AsyncReadExt;
    let mut child = tokio::process::Command::new("/bin/echo")
        .arg("pty-test-output")
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn echo");
    let mut stdout = child.stdout.take().unwrap();
    let mut buf = String::new();
    stdout.read_to_string(&mut buf).await.expect("read stdout");
    assert!(buf.contains("pty-test-output"), "got: {buf}");
    let status = child.wait().await.expect("wait");
    assert!(status.success());
}

// ── JSON message parsing ────────────────────────────────────────────

#[test]
fn resize_json_parsing() {
    let text = r#"{"type":"resize","cols":120,"rows":40}"#;
    let v: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(v.get("type").unwrap().as_str(), Some("resize"));
    assert_eq!(v.get("cols").unwrap().as_u64(), Some(120));
    assert_eq!(v.get("rows").unwrap().as_u64(), Some(40));
}

#[test]
fn non_resize_json_is_stdin() {
    let text = r#"{"type":"data","content":"ls"}"#;
    let v: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_ne!(v.get("type").unwrap().as_str(), Some("resize"));
}

#[test]
fn plain_text_is_stdin() {
    let text = "ls -la\n";
    assert!(serde_json::from_str::<serde_json::Value>(text).is_err());
}
