/// Real plan preflight checks: TCP reachability, SSH auth, heartbeat freshness.
/// Streams SSE events per peer: checking -> check (ok/fail) -> done summary.
use crate::mesh::peers::PeersRegistry;
use axum::response::sse::Event;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use serde_json::json;
use std::convert::Infallible;
use std::path::Path;
use std::time::Duration;

const TCP_PORT: u16 = 9420;
const TCP_TIMEOUT: Duration = Duration::from_secs(5);
const HEARTBEAT_STALE_SECS: i64 = 300;

type SseEvent = Result<Event, Infallible>;
type Conn = PooledConnection<SqliteConnectionManager>;

fn status_str(ok: bool) -> &'static str {
    if ok {
        "ok"
    } else {
        "fail"
    }
}

/// Load peers from ~/.claude/config/peers.conf, filtering to active only.
/// Returns empty vec when the file is missing.
pub fn load_active_peers(target: &str) -> Vec<(String, String)> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    let conf_path = format!("{home}/.claude/config/peers.conf");
    let registry = match PeersRegistry::load(Path::new(&conf_path)) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if target != "all" {
        if let Some(peer) = registry.peers.get(target) {
            if peer.status == "active" {
                return vec![(target.to_string(), peer.tailscale_ip.clone())];
            }
        }
        return Vec::new();
    }
    registry
        .list_active()
        .into_iter()
        .map(|(name, cfg)| (name.to_string(), cfg.tailscale_ip.clone()))
        .collect()
}

/// TCP connect test to peer_ip:9420 with 5s timeout (blocking).
fn tcp_check(peer_ip: &str) -> Result<(), String> {
    let addr_str = format!("{peer_ip}:{TCP_PORT}");
    let addr: std::net::SocketAddr = addr_str
        .parse()
        .map_err(|e| format!("invalid address {addr_str}: {e}"))?;
    std::net::TcpStream::connect_timeout(&addr, TCP_TIMEOUT)
        .map(|_| ())
        .map_err(|e| format!("{addr_str} unreachable: {e}"))
}

/// SSH auth test via ssh2 crate. Connects to peer_ip:22 and tries agent auth.
fn ssh_check(peer_ip: &str) -> Result<(), String> {
    let addr_str = format!("{peer_ip}:22");
    let addr: std::net::SocketAddr = addr_str
        .parse()
        .map_err(|e| format!("invalid SSH address: {e}"))?;
    let stream = std::net::TcpStream::connect_timeout(&addr, TCP_TIMEOUT)
        .map_err(|e| format!("SSH connect to {addr_str} failed: {e}"))?;
    let mut sess = ssh2::Session::new().map_err(|e| format!("SSH session create: {e}"))?;
    sess.set_tcp_stream(stream);
    sess.set_timeout(TCP_TIMEOUT.as_millis() as u32);
    sess.handshake()
        .map_err(|e| format!("SSH handshake with {peer_ip}: {e}"))?;
    let user = std::env::var("USER").unwrap_or_else(|_| "root".to_string());
    sess.userauth_agent(&user)
        .map_err(|e| format!("SSH agent auth as {user}@{peer_ip}: {e}"))?;
    if sess.authenticated() {
        Ok(())
    } else {
        Err(format!("SSH auth failed for {user}@{peer_ip}"))
    }
}

/// Check peer_heartbeats for recent activity (< 5 min).
fn heartbeat_check(conn: &Conn, peer_name: &str) -> Result<(), String> {
    let sql = "SELECT last_seen FROM peer_heartbeats WHERE peer_name = ?1 \
               ORDER BY last_seen DESC LIMIT 1";
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("heartbeat query: {e}"))?;
    let last_seen: Option<i64> = stmt
        .query_row(rusqlite::params![peer_name], |row| row.get(0))
        .ok();
    match last_seen {
        None => Err(format!("no heartbeat recorded for {peer_name}")),
        Some(ts) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let age = now - ts;
            if age <= HEARTBEAT_STALE_SECS {
                Ok(())
            } else {
                Err(format!(
                    "last heartbeat {age}s ago (threshold {HEARTBEAT_STALE_SECS}s)"
                ))
            }
        }
    }
}

/// Query plan status from DB. Returns (status_string, is_actionable).
pub fn plan_status_check(conn: &Conn, plan_id: &str) -> (String, bool) {
    let sql = "SELECT status FROM plans WHERE id = ?1";
    let status: Option<String> = conn
        .prepare(sql)
        .ok()
        .and_then(|mut s| s.query_row(rusqlite::params![plan_id], |r| r.get(0)).ok());
    match status {
        Some(ref s) if s == "todo" || s == "doing" => (format!("#{plan_id} is '{s}'"), true),
        Some(s) => (format!("#{plan_id} is '{s}' (not actionable)"), false),
        None => (format!("#{plan_id} not found in DB"), false),
    }
}

/// Push a checking + check event pair for one check on one peer.
fn push_check(
    events: &mut Vec<SseEvent>,
    all_ok: &mut bool,
    peer: &str,
    check: &str,
    result: Result<(), String>,
    ok_detail: &str,
) {
    events.push(Ok(Event::default()
        .event("checking")
        .data(json!({"peer": peer, "check": check}).to_string())));
    let (ok, detail) = match result {
        Ok(()) => (true, ok_detail.to_string()),
        Err(e) => {
            *all_ok = false;
            (false, e)
        }
    };
    events.push(Ok(Event::default().event("check").data(
        json!({"peer": peer, "check": check, "status": status_str(ok), "detail": detail})
            .to_string(),
    )));
}

/// Build full SSE events for a preflight check.
/// Runs TCP, SSH, and heartbeat checks for each active peer, plus plan status.
pub fn build_preflight_events(conn: &Conn, plan_id: &str, target: &str) -> Vec<SseEvent> {
    let peers = load_active_peers(target);
    let total_checks = peers.len() * 3 + 1;
    let mut events: Vec<SseEvent> = Vec::with_capacity(total_checks * 2 + 2);
    let mut all_ok = true;

    events.push(Ok(Event::default().event("start").data(
        json!({"plan_id": plan_id, "target": target, "total_checks": total_checks}).to_string(),
    )));

    for (name, ip) in &peers {
        push_check(
            &mut events,
            &mut all_ok,
            name,
            "tcp",
            tcp_check(ip),
            &format!("{ip}:{TCP_PORT} reachable"),
        );
        push_check(
            &mut events,
            &mut all_ok,
            name,
            "ssh",
            ssh_check(ip),
            "SSH agent auth succeeded",
        );
        push_check(
            &mut events,
            &mut all_ok,
            name,
            "heartbeat",
            heartbeat_check(conn, name),
            "heartbeat within 5min",
        );
    }

    // Plan status check
    let (plan_detail, plan_ok) = plan_status_check(conn, plan_id);
    if !plan_ok {
        all_ok = false;
    }
    push_check(
        &mut events,
        &mut all_ok,
        "db",
        "plan_status",
        if plan_ok {
            Ok(())
        } else {
            Err(plan_detail.clone())
        },
        &plan_detail,
    );

    let failed = if all_ok {
        0
    } else {
        events
            .iter()
            .filter(|e| {
                e.as_ref()
                    .map_or(false, |ev| format!("{ev:?}").contains(r#""status":"fail""#))
            })
            .count()
    };
    let passed = total_checks - failed;
    events.push(Ok(Event::default().event("done").data(
        json!({"ok": all_ok, "passed": passed, "failed": failed, "total": total_checks})
            .to_string(),
    )));
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_str_returns_correct_values() {
        assert_eq!(status_str(true), "ok");
        assert_eq!(status_str(false), "fail");
    }

    #[test]
    fn tcp_check_unreachable_returns_error() {
        // RFC 5737 TEST-NET address — guaranteed unreachable
        let result = tcp_check("192.0.2.1");
        assert!(result.is_err());
    }
}
