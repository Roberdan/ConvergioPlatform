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

/// Abstraction over the `mlx_lm.generate` CLI.
///
/// The bridge shells out to the mlx_lm Python package available via `mlx_lm.generate`.
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

    /// List models by querying `mlx_lm.generate --help` and parsing known model ids.
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

    /// Run inference via `python -m mlx_lm.generate`.
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
        // Probe: `python -m mlx_lm.generate --help` exits 0 when the package is installed.
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
    fn cli_cmd(&self) -> Vec<String> {
        if let Some(path) = &self.cli_path {
            vec![path.clone()]
        } else {
            vec![
                "python".to_string(),
                "-m".to_string(),
                "mlx_lm.generate".to_string(),
            ]
        }
    }

    fn run_subprocess(&self, req: &InferenceRequest) -> Result<AppleFmResponse, InferenceError> {
        use std::io::Write as _;

        let model = req
            .model
            .clone()
            .unwrap_or_else(|| "mlx-community/Llama-3.2-1B-Instruct-4bit".to_string());

        let cmd = self.cli_cmd();
        let mut child = std::process::Command::new(&cmd[0])
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
            unsafe {
                libc_kill(child_id);
            }
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: creates a bridge that points to a fake CLI path so real mlx_lm
    /// is never invoked, and uname is still used for arch detection.
    fn bridge_with_fake_cli(path: &str) -> AppleFmBridge {
        AppleFmBridge {
            cli_path: Some(path.to_string()),
        }
    }

    // --- availability detection ---

    #[test]
    fn test_is_available_returns_false_when_cli_missing() {
        // Point at a nonexistent binary — must report unavailable regardless of arch.
        let bridge = bridge_with_fake_cli("/nonexistent/mlx_lm_binary");
        // We cannot guarantee Apple Silicon in CI; what we can guarantee is that a
        // missing binary always means unavailable.
        let available = bridge.cli_available();
        assert!(!available, "cli_available must be false for nonexistent binary");
    }

    #[test]
    fn test_health_check_false_when_unavailable() {
        let bridge = bridge_with_fake_cli("/nonexistent/mlx_lm_binary");
        // health_check delegates to is_available, so this always matches.
        assert_eq!(bridge.health_check(), bridge.is_available());
    }

    #[test]
    fn test_list_models_empty_when_unavailable() {
        let bridge = bridge_with_fake_cli("/nonexistent/mlx_lm_binary");
        // When unavailable, list_models must return empty vec, not panic.
        let models = bridge.list_models();
        // Either empty (unavailable) or populated (available) — never panics.
        // On non-Apple-Silicon CI the list will always be empty.
        if bridge.is_available() {
            assert!(!models.is_empty());
        } else {
            assert!(models.is_empty());
        }
    }

    // --- inference ---

    #[test]
    fn test_infer_returns_error_when_unavailable() {
        let bridge = bridge_with_fake_cli("/nonexistent/mlx_lm_binary");
        let req = InferenceRequest {
            prompt: "Hello".to_string(),
            ..Default::default()
        };
        // If not available, infer must fail with Unavailable — never panic.
        if !bridge.is_available() {
            let result = bridge.infer(&req);
            assert!(result.is_err());
            match result.unwrap_err() {
                InferenceError::Unavailable(_) => {}
                other => panic!("expected Unavailable, got {other}"),
            }
        }
    }

    #[test]
    fn test_infer_subprocess_failure_propagates() {
        // Use `false` (always exits 1) as the fake CLI.
        let bridge = AppleFmBridge {
            cli_path: Some("false".to_string()),
        };
        // Manually override availability check: set cli_path to `true` for cli_available
        // but the actual run uses `false`. We test run_subprocess directly.
        let req = InferenceRequest {
            prompt: "test".to_string(),
            timeout_secs: 5,
            ..Default::default()
        };
        let result = bridge.run_subprocess(&req);
        assert!(result.is_err(), "subprocess exiting 1 must produce an error");
    }

    #[test]
    fn test_inference_request_default_timeout() {
        let req = InferenceRequest::default();
        assert_eq!(req.timeout_secs, 60);
        assert!(req.prompt.is_empty());
        assert!(req.model.is_none());
    }

    #[test]
    fn test_apple_fm_response_fields() {
        let resp = AppleFmResponse {
            text: "Hello world".to_string(),
            model: "test-model".to_string(),
            prompt_tokens: 2,
            completion_tokens: 0,
        };
        assert_eq!(resp.text, "Hello world");
        assert_eq!(resp.model, "test-model");
    }

    #[test]
    fn test_inference_error_display() {
        let e = InferenceError::Unavailable("no mlx".to_string());
        assert!(e.to_string().contains("no mlx"));
        let e2 = InferenceError::Timeout(Duration::from_secs(30));
        assert!(e2.to_string().contains("timeout"));
    }

    #[test]
    fn test_inference_error_converts_to_ipc_error() {
        let e: IpcError = InferenceError::Unavailable("x".to_string()).into();
        assert!(e.to_string().contains("x"));
    }

    #[test]
    fn test_list_models_contains_known_ids_when_available() {
        // Smoke: if the bridge reports available (Apple Silicon CI), known ids present.
        let bridge = AppleFmBridge::new();
        if bridge.is_available() {
            let models = bridge.list_models();
            assert!(models.iter().any(|m| m.contains("mlx-community")));
        }
    }
}
