// SSH helpers for SSE delegate streaming.
// Extracted from sse_delegate.rs for the 250-line limit.

use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;

pub fn ssh_connect(dest: &str) -> Option<Session> {
    let (user, host_port) = match dest.split_once('@') {
        Some((u, rest)) => (u.to_string(), rest.to_string()),
        None => (String::new(), dest.to_string()),
    };
    let addr = if host_port.contains(':') { host_port } else { format!("{host_port}:22") };
    let tcp = TcpStream::connect_timeout(&addr.parse().ok()?, Duration::from_secs(10)).ok()?;
    let _ = tcp.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = tcp.set_write_timeout(Some(Duration::from_secs(30)));
    let mut session = Session::new().ok()?;
    session.set_tcp_stream(tcp);
    session.handshake().ok()?;
    let auth_user = if user.is_empty() {
        std::env::var("USER").unwrap_or_else(|_| "root".to_string())
    } else {
        user
    };
    session.userauth_agent(&auth_user).ok()?;
    if !session.authenticated() {
        return None;
    }
    Some(session)
}

pub fn ssh_exec(session: &Session, cmd: &str) -> Result<(i32, String, String), String> {
    let mut ch = session.channel_session().map_err(|e| e.to_string())?;
    ch.exec(cmd).map_err(|e| e.to_string())?;
    let mut stdout = String::new();
    let mut stderr = String::new();
    ch.read_to_string(&mut stdout).map_err(|e| e.to_string())?;
    ch.stderr()
        .read_to_string(&mut stderr)
        .map_err(|e| e.to_string())?;
    let _ = ch.wait_close();
    let code = ch.exit_status().unwrap_or(-1);
    drop(ch);
    Ok((code, stdout, stderr))
}

pub fn ssh_git_sync(session: &Session, plan_id: &str) -> Result<(), String> {
    let branch = format!("plan-{plan_id}");
    let cmd = format!(
        "cd ~/GitHub/ConvergioPlatform && git fetch origin && \
         (git checkout {branch} 2>/dev/null || \
         git checkout -b {branch} origin/{branch} 2>/dev/null || true) && \
         git pull --ff-only origin {branch} 2>/dev/null || true"
    );
    let (code, _, stderr) = ssh_exec(session, &cmd)?;
    if code != 0 && !stderr.contains("Already on") && !stderr.contains("Already up to date") {
        return Err(format!("git sync exit {code}: {stderr}"));
    }
    Ok(())
}
