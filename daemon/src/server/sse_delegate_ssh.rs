// SSH helpers for SSE delegate streaming.
// Extracted from sse_delegate.rs for the 250-line limit.

use super::super::state::ApiError;
use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;

pub fn ssh_connect(dest: &str) -> Option<Session> {
    let (user, host_port) = match dest.split_once('@') {
        Some((u, rest)) => (u.to_string(), rest.to_string()),
        None => (String::new(), dest.to_string()),
    };
    let addr = if host_port.contains(':') {
        host_port
    } else {
        format!("{host_port}:22")
    };
    let parsed_addr = match addr.parse() {
        Ok(a) => a,
        Err(e) => { tracing::warn!("ssh addr parse failed: {e}"); return None; }
    };
    let tcp = match TcpStream::connect_timeout(&parsed_addr, Duration::from_secs(10)) {
        Ok(t) => t,
        Err(e) => { tracing::warn!("ssh connect failed to {addr}: {e}"); return None; }
    };
    if let Err(e) = tcp.set_read_timeout(Some(Duration::from_secs(30))) {
        tracing::warn!("ssh set_read_timeout failed: {e}");
    }
    if let Err(e) = tcp.set_write_timeout(Some(Duration::from_secs(30))) {
        tracing::warn!("ssh set_write_timeout failed: {e}");
    }
    let mut session = match Session::new() {
        Ok(s) => s,
        Err(e) => { tracing::warn!("ssh session create failed: {e}"); return None; }
    };
    session.set_tcp_stream(tcp);
    if let Err(e) = session.handshake() {
        tracing::warn!("ssh handshake failed: {e}"); return None;
    }
    let auth_user = if user.is_empty() {
        std::env::var("USER").unwrap_or_else(|_| "root".to_string())
    } else {
        user
    };
    if let Err(e) = session.userauth_agent(&auth_user) {
        tracing::warn!("ssh auth failed: {e}"); return None;
    }
    if !session.authenticated() {
        return None;
    }
    Some(session)
}

pub fn ssh_exec(session: &Session, cmd: &str) -> Result<(i32, String, String), ApiError> {
    let mut ch = session
        .channel_session()
        .map_err(|e| ApiError::internal(format!("ssh channel: {e}")))?;
    ch.exec(cmd)
        .map_err(|e| ApiError::internal(format!("ssh exec: {e}")))?;
    let mut stdout = String::new();
    let mut stderr = String::new();
    ch.read_to_string(&mut stdout)
        .map_err(|e| ApiError::internal(format!("ssh stdout read: {e}")))?;
    ch.stderr()
        .read_to_string(&mut stderr)
        .map_err(|e| ApiError::internal(format!("ssh stderr read: {e}")))?;
    if let Err(e) = ch.wait_close() {
        tracing::debug!("ssh channel wait_close: {e}");
    }
    let code = ch.exit_status().unwrap_or(-1);
    drop(ch);
    Ok((code, stdout, stderr))
}

pub fn ssh_git_sync(session: &Session, plan_id: &str) -> Result<(), ApiError> {
    let branch = format!("plan-{plan_id}");
    let cmd = format!(
        "cd ~/GitHub/ConvergioPlatform && git fetch origin && \
         (git checkout {branch} 2>/dev/null || \
         git checkout -b {branch} origin/{branch} 2>/dev/null || true) && \
         git pull --ff-only origin {branch} 2>/dev/null || true"
    );
    let (code, _, stderr) = ssh_exec(session, &cmd)?;
    if code != 0 && !stderr.contains("Already on") && !stderr.contains("Already up to date") {
        return Err(ApiError::internal(format!(
            "git sync exit {code}: {stderr}"
        )));
    }
    Ok(())
}
