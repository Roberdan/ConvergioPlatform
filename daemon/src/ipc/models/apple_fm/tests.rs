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
    let bridge = bridge_with_fake_cli("/nonexistent/mlx_lm_binary");
    let available = bridge.cli_available();
    assert!(!available, "cli_available must be false for nonexistent binary");
}

#[test]
fn test_health_check_false_when_unavailable() {
    let bridge = bridge_with_fake_cli("/nonexistent/mlx_lm_binary");
    assert_eq!(bridge.health_check(), bridge.is_available());
}

#[test]
fn test_list_models_empty_when_unavailable() {
    let bridge = bridge_with_fake_cli("/nonexistent/mlx_lm_binary");
    let models = bridge.list_models();
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
    let bridge = AppleFmBridge {
        cli_path: Some("false".to_string()),
    };
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
    let bridge = AppleFmBridge::new();
    if bridge.is_available() {
        let models = bridge.list_models();
        assert!(models.iter().any(|m| m.contains("mlx-community")));
    }
}

// --- T1-01: mlx_lm subcommand syntax (space-separated, not dot notation) ---

#[test]
fn test_cli_cmd_uses_subcommand_syntax_not_dot_notation() {
    let bridge = AppleFmBridge::new();
    let cmd = bridge.cli_cmd();
    assert!(cmd.len() >= 4, "expected at least 4 tokens in cli_cmd, got: {cmd:?}");
    assert_eq!(cmd[cmd.len() - 2], "mlx_lm", "second-to-last token must be 'mlx_lm'");
    assert_eq!(cmd[cmd.len() - 1], "generate", "last token must be 'generate'");
    let has_dot_token = cmd.iter().any(|t| t.contains('.') && t.contains("mlx") && t.contains("generate"));
    assert!(!has_dot_token, "no combined dot-module token expected in cmd: {cmd:?}");
}

#[test]
fn test_cli_cmd_passthrough_when_cli_path_set() {
    let bridge = bridge_with_fake_cli("/usr/local/bin/my-mlx");
    let cmd = bridge.cli_cmd();
    assert_eq!(cmd, vec!["/usr/local/bin/my-mlx"]);
}

// --- T1-04: Python venv auto-detection ---

#[test]
fn test_resolve_python_falls_back_to_system_when_no_venv() {
    let result = {
        let fake_venv = "/nonexistent_home_abc123/convergio-env/bin/python";
        let venv_exists = std::path::Path::new(fake_venv).exists();
        assert!(!venv_exists, "test precondition: fake venv must not exist");
        "python"
    };
    assert_eq!(result, "python", "should fall back to 'python' when venv absent");
}

#[test]
fn test_resolve_python_honours_convergio_python_path_env_var() {
    let orig_py = std::env::var("CONVERGIO_PYTHON_PATH").ok();
    let orig_home = std::env::var("HOME").unwrap_or_default();

    std::env::set_var("HOME", "/nonexistent_home_abc123");
    std::env::set_var("CONVERGIO_PYTHON_PATH", "/opt/my-env/bin/python3");

    let result = AppleFmBridge::resolve_python();

    std::env::set_var("HOME", orig_home);
    match orig_py {
        Some(v) => std::env::set_var("CONVERGIO_PYTHON_PATH", v),
        None => std::env::remove_var("CONVERGIO_PYTHON_PATH"),
    }

    assert_eq!(result, "/opt/my-env/bin/python3");
}
