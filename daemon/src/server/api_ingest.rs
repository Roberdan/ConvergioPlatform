// api_ingest: document/URL ingestion endpoints
// Spawns scripts/platform/convergio-ingest.sh for actual conversion work.
use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::process::Command;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/ingest", post(ingest_source))
        .route("/api/ingest/formats", get(ingest_formats))
}

#[derive(Deserialize)]
struct IngestBody {
    source: String,
    output_dir: String,
}

async fn ingest_source(
    State(_state): State<ServerState>,
    Json(body): Json<IngestBody>,
) -> Result<Json<Value>, ApiError> {
    let source = body.source.trim().to_string();
    let output_dir = body.output_dir.trim().to_string();

    if source.is_empty() {
        return Err(ApiError::bad_request("source is required"));
    }
    if output_dir.is_empty() {
        return Err(ApiError::bad_request("output_dir is required"));
    }

    // Locate ingest script relative to repo root (daemon cwd or CARGO_MANIFEST_DIR)
    let script = find_ingest_script();

    let output = Command::new("bash")
        .arg(&script)
        .arg("--source")
        .arg(&source)
        .arg("--output-dir")
        .arg(&output_dir)
        .output()
        .await
        .map_err(|e| ApiError::internal(format!("failed to spawn ingest script: {e}")))?;

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let ok = output.status.success();

    Ok(Json(json!({
        "ok": ok,
        "output_dir": output_dir,
        "stdout": stdout,
        "stderr": stderr,
        "exit_code": exit_code,
    })))
}

async fn ingest_formats(
    State(_state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    // Check which conversion tools are available on the system PATH.
    let pdf = tool_available("pdftotext").await;
    let docx = tool_available("pandoc").await;
    let url = tool_available("trafilatura").await;
    // pandoc handles both docx and pptx; xlsx conversion uses python-docx / openpyxl via the script
    let xlsx = tool_available("pandoc").await;
    let pptx = tool_available("pandoc").await;

    Ok(Json(json!({
        "pdf":    pdf,
        "docx":   docx,
        "url":    url,
        "xlsx":   xlsx,
        "pptx":   pptx,
        "images": true,   // always advertised; OCR via tesseract is optional at runtime
    })))
}

/// Returns true if `name` is found on PATH via `command -v`.
async fn tool_available(name: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {name}"))
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Locate scripts/platform/convergio-ingest.sh from either:
/// 1. CARGO_MANIFEST_DIR (test / cargo run from daemon/)
/// 2. Current working directory (daemon process run from repo root)
fn find_ingest_script() -> String {
    let candidates = [
        // When running from daemon/ directory
        "../scripts/platform/convergio-ingest.sh",
        // When running from repo root
        "scripts/platform/convergio-ingest.sh",
        // Absolute via manifest dir
        &format!(
            "{}/../../scripts/platform/convergio-ingest.sh",
            env!("CARGO_MANIFEST_DIR")
        ),
    ];
    for c in &candidates {
        if std::path::Path::new(c).exists() {
            return c.to_string();
        }
    }
    // Fall back to name only — will fail loudly if not on PATH
    "convergio-ingest.sh".to_string()
}
