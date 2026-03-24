// Persistent Claude CLI session — NDJSON over stdin/stdout.
// Spawns `claude` once, reuses for all chat messages. Streams token deltas.

use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

use crate::tui::session_lifecycle::{mcp_config_path, process_ndjson_line};

/// Events emitted by the Claude session reader task.
#[derive(Debug, Clone)]
pub enum ChatEvent {
    /// Incremental text delta (token-level streaming).
    TextDelta(String),
    /// Complete assistant message (final aggregated text).
    MessageComplete(String),
    /// Session initialization with session_id.
    SessionReady(String),
    /// Error from the session process.
    Error(String),
}

/// Manages a persistent `claude` subprocess with NDJSON I/O.
pub struct ClaudeSession {
    child: Child,
    stdin: Option<tokio::process::ChildStdin>,
    pub event_rx: mpsc::UnboundedReceiver<ChatEvent>,
}

impl ClaudeSession {
    /// Spawn a persistent `claude` session with stream-json I/O.
    pub fn spawn() -> Result<Self, String> {
        let mut child = Command::new("claude")
            .args([
                "--input-format", "stream-json",
                "--output-format", "stream-json",
                "--verbose",
                "--agent", "ali-chief-of-staff",
                "--model", "opus",
                "--dangerously-skip-permissions",
                "--mcp-config", &mcp_config_path(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("failed to spawn claude: {e}"))?;

        let stdin = child.stdin.take();
        let stdout = child.stdout.take()
            .ok_or_else(|| "failed to capture claude stdout".to_string())?;

        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Reader task: parse NDJSON lines from stdout.
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut partial = String::new();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                let parsed: serde_json::Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                process_ndjson_line(&parsed, &mut partial, &event_tx);
            }

            // Process exited — notify.
            let _ = event_tx.send(ChatEvent::Error("Claude session ended".to_string()));
        });

        Ok(Self { child, stdin, event_rx })
    }

    /// Send a user message to the persistent session.
    pub async fn send(&mut self, content: &str) -> Result<(), String> {
        let stdin = self.stdin.as_mut()
            .ok_or_else(|| "stdin not available".to_string())?;

        let msg = serde_json::json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": [{"type": "text", "text": content}]
            }
        });
        let mut line = serde_json::to_string(&msg)
            .map_err(|e| format!("json serialize: {e}"))?;
        line.push('\n');

        stdin.write_all(line.as_bytes()).await
            .map_err(|e| format!("write to claude stdin: {e}"))?;
        stdin.flush().await
            .map_err(|e| format!("flush claude stdin: {e}"))?;

        Ok(())
    }

    /// Check if the subprocess is still running.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

impl Drop for ClaudeSession {
    fn drop(&mut self) {
        // Best-effort kill on drop.
        let _ = self.child.start_kill();
    }
}
