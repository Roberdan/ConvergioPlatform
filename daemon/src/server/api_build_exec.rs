use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::Instant;
use tokio::process::Command;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/build", post(handle_build))
        .route("/api/build/status", get(handle_build_status))
        .route("/api/test", post(handle_test))
        .route("/api/test/status", get(handle_test_status))
}

#[derive(Default, Clone)]
struct JobState {
    running: bool,
    last: Value,
}

#[derive(Deserialize)]
struct ExecBody {
    cwd: Option<String>,
    suite: Option<String>,
    no_cache: Option<bool>,
    compact: Option<bool>,
}

static BUILD_STATE: LazyLock<Mutex<JobState>> = LazyLock::new(|| Mutex::new(JobState::default()));
static TEST_STATE: LazyLock<Mutex<JobState>> = LazyLock::new(|| Mutex::new(JobState::default()));

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
        .to_path_buf()
}

fn request_payload(cwd: &str, suite: Option<&str>, no_cache: bool, compact: bool) -> Value {
    json!({
        "cwd": cwd,
        "suite": suite.unwrap_or("all"),
        "no_cache": no_cache,
        "compact": compact,
    })
}

async fn launch_job(kind: &'static str, body: ExecBody) -> Result<Json<Value>, ApiError> {
    let state = match kind {
        "build" => &BUILD_STATE,
        _ => &TEST_STATE,
    };
    {
        let mut guard = state.lock().map_err(|_| ApiError::internal("job state poisoned"))?;
        if guard.running {
            return Err(ApiError::conflict(format!("{kind} already running")));
        }
        guard.running = true;
        guard.last = json!({"status":"running"});
    }
    let cwd = body.cwd.unwrap_or_else(|| repo_root().to_string_lossy().to_string());
    let suite = body.suite;
    let no_cache = body.no_cache.unwrap_or(false);
    let compact = body.compact.unwrap_or(false);
    let request = request_payload(&cwd, suite.as_deref(), no_cache, compact);
    tokio::spawn(async move {
        let started = Instant::now();
        let script_name = if kind == "build" {
            "build-digest.sh"
        } else {
            "test-digest.sh"
        };
        let mut cmd = Command::new("bash");
        cmd.arg(repo_root().join("claude-config/scripts").join(script_name))
            .current_dir(&cwd)
            .env("DIGEST_API_BYPASS", "1");
        if compact {
            cmd.arg("--compact");
        }
        if no_cache {
            cmd.arg("--no-cache");
        }
        if kind == "test" {
            if let Some(suite) = suite {
                cmd.args(["--suite", &suite]);
            }
        }
        let result = match cmd.output().await {
            Ok(output) if output.status.success() => serde_json::from_slice::<Value>(&output.stdout)
                .unwrap_or_else(|_| json!({"status":"error","error":"invalid json"})),
            Ok(output) => json!({
                "status":"error",
                "exit_code": output.status.code(),
                "stderr": String::from_utf8_lossy(&output.stderr),
            }),
            Err(e) => json!({"status":"error","error": e.to_string()}),
        };
        if let Ok(mut guard) = state.lock() {
            guard.running = false;
            guard.last = json!({
                "status": "completed",
                "kind": kind,
                "request": request,
                "duration_secs": started.elapsed().as_secs(),
                "result": result,
            });
        }
    });
    Ok(Json(json!({"ok": true, "status": "running", "kind": kind})))
}

fn status_json(kind: &'static str) -> Result<Json<Value>, ApiError> {
    let state = match kind {
        "build" => &BUILD_STATE,
        _ => &TEST_STATE,
    };
    let guard = state.lock().map_err(|_| ApiError::internal("job state poisoned"))?;
    Ok(Json(json!({
        "ok": true,
        "kind": kind,
        "running": guard.running,
        "last": guard.last,
    })))
}

async fn handle_build(
    State(_state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let body: ExecBody = serde_json::from_value(body).unwrap_or(ExecBody {
        cwd: None,
        suite: None,
        no_cache: None,
        compact: None,
    });
    launch_job("build", body).await
}

async fn handle_test(
    State(_state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let body: ExecBody = serde_json::from_value(body).unwrap_or(ExecBody {
        cwd: None,
        suite: None,
        no_cache: None,
        compact: None,
    });
    launch_job("test", body).await
}

async fn handle_build_status(State(_state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    status_json("build")
}

async fn handle_test_status(State(_state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    status_json("test")
}
