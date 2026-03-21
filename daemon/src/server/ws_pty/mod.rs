//! WebSocket PTY terminal — spawns local shell or SSH to remote mesh nodes.
//! Protocol: JSON `{"type":"resize","cols":N,"rows":N}` for resize, raw text for stdin.
pub mod session;

use super::state::ServerState;
pub(crate) use session::{
    is_local_peer, peer_ssh_alias, peer_ssh_user, tailscale_resolve, validate_peer,
};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use serde::Deserialize;
use session::{load_known_peers, MAX_TMUX_SESSION_LEN};
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{Duration, Instant};

pub(crate) static ACTIVE_SESSIONS: AtomicUsize = AtomicUsize::new(0);
pub(crate) const MAX_PTY_SESSIONS: usize = 10;
const IDLE_TIMEOUT: Duration = Duration::from_secs(300);

struct SessionGuard;
impl Drop for SessionGuard {
    fn drop(&mut self) {
        ACTIVE_SESSIONS.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Deserialize)]
pub struct PtyParams {
    #[serde(default = "default_peer")]
    pub(crate) peer: String,
    #[serde(default)]
    pub(crate) tmux_session: String,
}
fn default_peer() -> String {
    "local".into()
}

pub(crate) fn sanitize_tmux_session(session: &str) -> Option<String> {
    if session.is_empty() {
        return Some(String::new());
    }
    if session.len() > MAX_TMUX_SESSION_LEN {
        return None;
    }
    if session
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        Some(session.to_string())
    } else {
        None
    }
}

fn validate_pty_params(params: PtyParams) -> Result<PtyParams, String> {
    let known_peers = load_known_peers();
    validate_peer(&params.peer, &known_peers)?;
    let tmux_session = sanitize_tmux_session(&params.tmux_session)
        .ok_or_else(|| "invalid tmux_session: only [A-Za-z0-9_-], max 64 chars".to_string())?;
    Ok(PtyParams {
        peer: params.peer,
        tmux_session,
    })
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub(crate) fn build_pty_command(
    state: &ServerState,
    params: &PtyParams,
    is_local: bool,
) -> (String, Vec<String>) {
    if is_local {
        if params.tmux_session.is_empty() {
            let sh = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
            (sh, vec!["-l".into()])
        } else {
            (
                "tmux".into(),
                vec![
                    "new-session".into(),
                    "-A".into(),
                    "-s".into(),
                    params.tmux_session.clone(),
                ],
            )
        }
    } else {
        let host = peer_ssh_alias(state, &params.peer).unwrap_or_else(|| params.peer.clone());
        if params.tmux_session.is_empty() {
            (
                "ssh".into(),
                vec!["-tt".into(), host, "exec $SHELL -l".into()],
            )
        } else {
            let tmux_cmd = format!("tmux new-session -A -s {}", params.tmux_session);
            let cmd = format!("exec $SHELL -lc {}", shell_single_quote(&tmux_cmd));
            ("ssh".into(), vec!["-tt".into(), host, cmd])
        }
    }
}

pub async fn ws_pty(
    ws: WebSocketUpgrade,
    Query(params): Query<PtyParams>,
    State(state): State<ServerState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_pty(socket, params, state))
}

async fn handle_pty(mut socket: WebSocket, params: PtyParams, state: ServerState) {
    let params = match validate_pty_params(params) {
        Ok(p) => p,
        Err(err) => {
            let _ = socket
                .send(Message::Text(format!("Invalid PTY params: {err}")))
                .await;
            return;
        }
    };
    let prev = ACTIVE_SESSIONS.fetch_add(1, Ordering::SeqCst);
    if prev >= MAX_PTY_SESSIONS {
        ACTIVE_SESSIONS.fetch_sub(1, Ordering::SeqCst);
        let _ = socket
            .send(Message::Text(format!(
                "Max PTY sessions ({MAX_PTY_SESSIONS}) reached"
            )))
            .await;
        return;
    }
    let _guard = SessionGuard;
    let is_local = is_local_peer(&state, &params.peer);
    let (program, args) = build_pty_command(&state, &params, is_local);
    let mut child = match tokio::process::Command::new(&program)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("TERM", "xterm-256color")
        .kill_on_drop(true)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = socket
                .send(Message::Text(format!("Spawn error: {e}")))
                .await;
            return;
        }
    };
    let mut stdin = child.stdin.take().expect("stdin piped");
    let mut stdout = child.stdout.take().expect("stdout piped");
    let mut stderr = child.stderr.take().expect("stderr piped");
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
    let tx2 = tx.clone();
    tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        while let Ok(n) = stdout.read(&mut buf).await {
            if n == 0 || tx.send(buf[..n].to_vec()).await.is_err() {
                break;
            }
        }
    });
    tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        while let Ok(n) = stderr.read(&mut buf).await {
            if n == 0 || tx2.send(buf[..n].to_vec()).await.is_err() {
                break;
            }
        }
    });
    let mut last_activity = Instant::now();
    loop {
        tokio::select! {
            Some(data) = rx.recv() => {
                last_activity = Instant::now();
                if socket.send(Message::Binary(data)).await.is_err() { break; }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        last_activity = Instant::now();
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                            if v.get("type").and_then(|t| t.as_str()) == Some("resize") {
                                continue;
                            }
                        }
                        if stdin.write_all(text.as_bytes()).await.is_err() { break; }
                    }
                    Some(Ok(Message::Binary(data))) => {
                        last_activity = Instant::now();
                        if stdin.write_all(&data).await.is_err() { break; }
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    _ => {}
                }
            }
            _ = tokio::time::sleep_until(last_activity + IDLE_TIMEOUT) => {
                let _ = socket.send(Message::Text("Session timed out (5min idle)".into())).await;
                break;
            }
        }
    }
    drop(stdin);
    let _ = child.kill().await;
}
