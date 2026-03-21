// nightly: nightly job list/detail/retry/trigger handlers + helpers
// config/create/toggle handlers → nightly_handlers.rs
// optimize signal handlers → nightly_data.rs
pub use super::nightly_data::{api_optimize_clear, api_optimize_signals};
pub use super::nightly_handlers::{
    api_coordinator_status, api_coordinator_toggle, api_events, api_nightly_config_get,
    api_nightly_config_update, api_nightly_def_toggle, api_nightly_job_create,
};

use super::super::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::path::Path as FsPath;
use std::process::Command;

// --- helpers ---

fn parse_positive_i64(
    qs: &HashMap<String, String>,
    key: &str,
    default_value: i64,
) -> Result<i64, ApiError> {
    let value = qs
        .get(key)
        .map(|raw| {
            raw.parse::<i64>()
                .map_err(|_| ApiError::bad_request(format!("invalid {key}")))
        })
        .transpose()?
        .unwrap_or(default_value);
    if value < 1 {
        return Err(ApiError::bad_request(format!("{key} must be >= 1")));
    }
    Ok(value)
}

fn parse_json_text_field(row: &mut Value, field: &str) -> Result<(), ApiError> {
    let raw = row
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    if let Some(raw) = raw {
        let parsed = serde_json::from_str::<Value>(&raw)
            .map_err(|err| ApiError::internal(format!("invalid {field}: {err}")))?;
        if let Some(object) = row.as_object_mut() {
            object.insert(field.to_string(), parsed);
        }
    }
    Ok(())
}

fn spawn_nightly_guardian(project_id: &str, trigger_source: &str, parent_run_id: Option<&str>) {
    #[cfg(test)]
    {
        let _ = (project_id, trigger_source, parent_run_id);
    }

    #[cfg(not(test))]
    {
        let claude_home = env::var("CLAUDE_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                env::var("HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                    .join(".claude")
            });
        let script_name = format!("{project_id}-nightly-guardian.sh");
        let script_path = claude_home.join(format!("scripts/{script_name}"));
        if !script_path.exists() {
            eprintln!(
                "[api_dashboard] nightly guardian script not found: {} (project: {})",
                script_path.display(),
                project_id
            );
            return;
        }

        let mut command = Command::new(script_path);
        command
            .arg(format!("--trigger={trigger_source}"))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if let Some(parent_run_id) = parent_run_id.filter(|value| !value.is_empty()) {
            command.arg(format!("--parent-run-id={parent_run_id}"));
        }
        if let Err(err) = command.spawn() {
            eprintln!("[api_dashboard] failed to spawn nightly guardian for {project_id}: {err}");
        }
    }
}

// --- nightly job list/detail/retry/trigger handlers ---

pub async fn api_nightly_jobs(
    State(state): State<ServerState>,
    Query(qs): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let page = parse_positive_i64(&qs, "page", 1)?;
    let per_page = parse_positive_i64(&qs, "per_page", 50)?.min(100);
    let offset = (page - 1) * per_page;
    let list_sql = "SELECT id, run_id, job_name, started_at, finished_at, host, status,
            sentry_unresolved, github_open_issues, processed_items, fixed_items,
            branch_name, pr_url, summary, report_json,
            duration_sec, trigger_source, exit_code, error_detail, log_file_path, parent_run_id
        FROM nightly_jobs ORDER BY started_at DESC LIMIT ?1 OFFSET ?2";
    let fallback_list_sql = "SELECT id, run_id, 'guardian' AS job_name, started_at, finished_at, host, status,
            sentry_unresolved, github_open_issues, processed_items, fixed_items,
            branch_name, pr_url, summary, report_json,
            NULL AS duration_sec, 'scheduled' AS trigger_source, NULL AS exit_code, NULL AS error_detail,
            NULL AS log_file_path, NULL AS parent_run_id
        FROM nightly_jobs ORDER BY started_at DESC LIMIT ?1 OFFSET ?2";
    let rows = query_rows(&conn, list_sql, rusqlite::params![per_page, offset]).or_else(|_| {
        query_rows(&conn, fallback_list_sql, rusqlite::params![per_page, offset])
    })?;
    let latest = query_one(
        &conn,
        "SELECT id, run_id, job_name, started_at, finished_at, host, status,
            sentry_unresolved, github_open_issues, processed_items, fixed_items,
            branch_name, pr_url, summary, report_json,
            duration_sec, trigger_source, exit_code, error_detail, log_file_path, parent_run_id
        FROM nightly_jobs ORDER BY started_at DESC LIMIT 1",
        [],
    )
    .or_else(|_| {
        query_one(
            &conn,
            "SELECT id, run_id, 'guardian' AS job_name, started_at, finished_at, host, status,
                sentry_unresolved, github_open_issues, processed_items, fixed_items,
                branch_name, pr_url, summary, report_json,
                NULL AS duration_sec, 'scheduled' AS trigger_source, NULL AS exit_code, NULL AS error_detail,
                NULL AS log_file_path, NULL AS parent_run_id
            FROM nightly_jobs ORDER BY started_at DESC LIMIT 1",
            [],
        )
    })?;
    let total = query_one(&conn, "SELECT COUNT(*) AS total FROM nightly_jobs", [])?
        .and_then(|row| row.get("total").and_then(Value::as_i64))
        .unwrap_or(0);
    let definitions = query_rows(
        &conn,
        "SELECT id,name,description,schedule,script_path,target_host,enabled,created_at,project_id,run_fixes,timeout_sec FROM nightly_job_definitions ORDER BY name",
        [],
    ).unwrap_or_default();
    Ok(Json(json!({
        "ok": true,
        "latest": latest,
        "history": rows,
        "definitions": definitions,
        "page": page,
        "per_page": per_page,
        "total": total
    })))
}

pub async fn api_nightly_job_detail(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let mut row = query_one(
        &conn,
        "SELECT id, run_id, job_name, started_at, finished_at, host, status,
            sentry_unresolved, github_open_issues, processed_items, fixed_items,
            branch_name, pr_url, summary, report_json,
            duration_sec, trigger_source, exit_code, error_detail, log_file_path, parent_run_id,
            log_stdout, log_stderr, config_snapshot
        FROM nightly_jobs WHERE id = ?1",
        rusqlite::params![id],
    )
    .or_else(|_| {
        query_one(
            &conn,
            "SELECT id, run_id, 'guardian' AS job_name, started_at, finished_at, host, status,
                sentry_unresolved, github_open_issues, processed_items, fixed_items,
                branch_name, pr_url, summary, report_json,
                NULL AS duration_sec, 'scheduled' AS trigger_source, NULL AS exit_code, NULL AS error_detail,
                NULL AS log_file_path, NULL AS parent_run_id,
                NULL AS log_stdout, NULL AS log_stderr, NULL AS config_snapshot
            FROM nightly_jobs WHERE id = ?1",
            rusqlite::params![id],
        )
    })?
    .ok_or_else(|| ApiError::bad_request(format!("nightly job {id} not found")))?;
    parse_json_text_field(&mut row, "report_json")?;
    parse_json_text_field(&mut row, "config_snapshot")?;
    let log_available = row
        .get("log_file_path")
        .and_then(Value::as_str)
        .map(|path| !path.is_empty() && FsPath::new(path).exists())
        .unwrap_or(false);
    if let Some(object) = row.as_object_mut() {
        object.insert("log_available".to_string(), Value::Bool(log_available));
    }
    Ok(Json(row))
}

#[derive(Deserialize)]
pub struct TriggerPayload {
    project_id: Option<String>,
}

pub async fn api_nightly_job_retry(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let original = query_one(
        &conn,
        "SELECT run_id, job_name FROM nightly_jobs WHERE id=?1",
        rusqlite::params![id],
    )?
    .ok_or_else(|| ApiError::bad_request(format!("nightly job {id} not found")))?;
    let parent_run_id = original
        .get("run_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let project_id = original
        .get("job_name")
        .and_then(Value::as_str)
        .and_then(|name| name.split('-').next())
        .unwrap_or("mirrorbuddy");
    spawn_nightly_guardian(project_id, "retry", parent_run_id.as_deref());
    Ok(Json(json!({"ok": true, "triggered": true, "parent_run_id": parent_run_id, "project_id": project_id})))
}

pub async fn api_nightly_job_trigger(
    axum::Json(payload): axum::Json<TriggerPayload>,
) -> Result<Json<Value>, ApiError> {
    let project_id = payload
        .project_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "mirrorbuddy".to_string());
    spawn_nightly_guardian(&project_id, "manual", None);
    Ok(Json(json!({"ok": true, "triggered": true, "project_id": project_id})))
}
