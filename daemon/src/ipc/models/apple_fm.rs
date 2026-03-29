use std::time::Duration;

use crate::ipc::error::IpcError;

/// Result of a single Apple Foundation Model inference call.
#[derive(Debug, Clone)]
pub struct AppleFmResponse {
    pub text: String,
    pub model: String,
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
}

/// Error variants specific to the Apple FM bridge.
#[derive(Debug, thiserror::Error)]
pub enum InferenceError {
    #[error("apple fm unavailable: {0}")]
    Unavailable(String),
    #[error("subprocess failed: {0}")]
    SubprocessFailed(String),
    #[error("timeout after {0:?}")]
    Timeout(Duration),
    #[error("parse error: {0}")]
    Parse(String),
}

impl From<InferenceError> for IpcError {
    fn from(e: InferenceError) -> Self {
        IpcError::Other(e.to_string())
    }
}

/// Minimal request forwarded to the on-device model.
#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub prompt: String,
    pub model: Option<String>,
    /// Wall-clock limit for the subprocess, in seconds.
    pub timeout_secs: u64,
}

impl Default for InferenceRequest {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            model: None,
            timeout_secs: 60,
        }
    }
}

/// Abstraction over the `mlx_lm generate` CLI.
///
/// The bridge shells out to the mlx_lm Python package via `python -m mlx_lm generate`
/// (subcommand/space syntax — the old dot-module form is deprecated and must not be used).
/// When the CLI is absent or the machine is not Apple Silicon the bridge reports
/// itself as unavailable; callers are expected to fall back to a cloud provider.
pub struct AppleFmBridge {
    /// Override the binary path for tests; `None` → resolve via PATH.
    pub(crate) cli_path: Option<String>,
}

impl Default for AppleFmBridge {
    fn default() -> Self {
        Self { cli_path: None }
    }
}

impl AppleFmBridge {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` when both `mlx_lm` CLI is reachable **and** we are on Apple Silicon.
    pub fn is_available(&self) -> bool {
        self.apple_silicon_detected() && self.cli_available()
    }

    /// Quick liveness probe — same as `is_available` but named for API symmetry with
    /// the ollama/lmstudio probes.
    pub fn health_check(&self) -> bool {
        self.is_available()
    }

    /// List models — `mlx_lm` has no list-models subcommand; returns a static set.
    ///
    /// When unavailable returns an empty vec rather than an error so callers can
    /// treat absence of models as a no-op.
    pub fn list_models(&self) -> Vec<String> {
        if !self.is_available() {
            return vec![];
        }
        // mlx_lm does not expose a list-models subcommand; we return a static set
        // of well-known model identifiers managed by scripts/kernel/setup-models.sh.
        vec![
            // Core inference models (required)
            "mlx-community/Mistral-7B-Instruct-v0.3-4bit".to_string(),
            "mlx-community/Qwen2.5-7B-Instruct-4bit".to_string(),
            "mlx-community/Codestral-22B-v0.1-4bit".to_string(),
            // Speech-to-text
            "mlx-community/whisper-small".to_string(),
            // Optional — may not be present on all installs
            "mlx-community/Mistral-Small-3.1-24B-Instruct-2503-4bit".to_string(),
            "mlx-community/Voxtral-Mini-3B-2507-4bit".to_string(),
        ]
    }

    /// Run inference via `python -m mlx_lm generate`.
    ///
    /// Shells out with a hard timeout; on timeout or subprocess failure returns
    /// `InferenceError` so the caller can trigger the cloud fallback.
    pub fn infer(&self, request: &InferenceRequest) -> Result<AppleFmResponse, InferenceError> {
        if !self.is_available() {
            return Err(InferenceError::Unavailable(
                "mlx_lm CLI not found or not on Apple Silicon".to_string(),
            ));
        }
        self.run_subprocess(request)
    }

    // --- private helpers ---

    fn apple_silicon_detected(&self) -> bool {
        // `uname -m` returns "arm64" on Apple Silicon Macs.
        std::process::Command::new("uname")
            .arg("-m")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "arm64")
            .unwrap_or(false)
    }

    fn cli_available(&self) -> bool {
        let cmd = self.cli_cmd();
        // Probe: `python -m mlx_lm generate --help` exits 0 when the package is installed.
        std::process::Command::new(&cmd[0])
            .args(&cmd[1..])
            .arg("--help")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Build the base command tokens depending on `cli_path`.
    ///
    /// Python resolution order (T1-04):
    ///   1. `~/convergio-env/bin/python` — venv used on M1 Pro installs
    ///   2. `CONVERGIO_PYTHON_PATH` env var — explicit override
    ///   3. `python` — system PATH fallback
    ///
    /// mlx_lm invocation uses subcommand syntax `python -m mlx_lm generate`
    /// (space-separated tokens, not the deprecated dot-module form — T1-01).
    fn cli_cmd(&self) -> Vec<String> {
        if let Some(path) = &self.cli_path {
            return vec![path.clone()];
        }

        // Resolve the Python interpreter.
        let python = Self::resolve_python();

        vec![python, "-m".to_string(), "mlx_lm".to_string(), "generate".to_string()]
    }

    /// Resolves the Python interpreter path with the venv-first strategy.
    pub fn resolve_python() -> String {
        // 1. Check for convergio venv (standard M1 Pro install location).
        if let Ok(home) = std::env::var("HOME") {
            let venv_python = format!("{home}/convergio-env/bin/python");
            if std::path::Path::new(&venv_python).exists() {
                return venv_python;
            }
        }

        // 2. Honour explicit env-var override.
        if let Ok(path) = std::env::var("CONVERGIO_PYTHON_PATH") {
            if !path.is_empty() {
                return path;
            }
        }

        // 3. Fall back to whatever `python` resolves to in PATH.
        "python".to_string()
    }

    fn run_subprocess(&self, req: &InferenceRequest) -> Result<AppleFmResponse, InferenceError> {
        let model = req
            .model
            .clone()
            .unwrap_or_else(|| "mlx-community/Llama-3.2-1B-Instruct-4bit".to_string());

        let cmd = self.cli_cmd();
        let child = std::process::Command::new(&cmd[0])
            .args(&cmd[1..])
            .args(["--model", &model, "--prompt", &req.prompt, "--max-tokens", "512"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| InferenceError::SubprocessFailed(e.to_string()))?;

        // Enforce wall-clock timeout via a background thread.
        let timeout = Duration::from_secs(req.timeout_secs);
        let child_id = child.id();
        let handle = std::thread::spawn(move || {
            std::thread::sleep(timeout);
            // SIGTERM the child if it is still running.
            #[cfg(unix)]
            libc_kill(child_id);
        });

        let output = child
            .wait_with_output()
            .map_err(|e| InferenceError::SubprocessFailed(e.to_string()))?;

        // Thread is detached; we do not need its result.
        drop(handle);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr.contains("killed") || stderr.contains("timeout") {
                return Err(InferenceError::Timeout(timeout));
            }
            return Err(InferenceError::SubprocessFailed(stderr));
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(AppleFmResponse {
            text,
            model,
            prompt_tokens: req.prompt.split_whitespace().count(),
            completion_tokens: 0, // mlx_lm does not report token counts in plain-text mode
        })
    }
}

// Safety: signal helper — send SIGTERM to a child process on Unix.
#[cfg(unix)]
fn libc_kill(pid: u32) {
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    unsafe {
        kill(pid as i32, 15 /* SIGTERM */);
    }
}

#[cfg(test)]
mod tests;

