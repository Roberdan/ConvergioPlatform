//! Memory file management API — list, stats, GC, delete for Claude project memory files.
//! Distinct from api_memory.rs which handles vector-embedding memory store.

use super::state::{ApiError, ServerState};
use axum::extract::{Path, Query};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/memory-mgmt/list", get(list_memories))
        .route("/api/memory-mgmt/stats", get(memory_stats))
        .route("/api/memory-mgmt/gc", post(garbage_collect))
        .route("/api/memory-mgmt/file/:filename", delete(delete_memory))
}

#[derive(Deserialize)]
struct MemoryQuery {
    slug: Option<String>,
}

/// Resolve memory directory. Uses CLAUDE_PROJECTS_DIR env for testability.
pub(crate) fn resolve_memory_dir(slug: Option<&str>) -> Result<PathBuf, ApiError> {
    let base = std::env::var("CLAUDE_PROJECTS_DIR").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        format!("{home}/.claude/projects")
    });
    let slug = match slug {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => {
            let cwd = std::env::current_dir()
                .map_err(|e| ApiError::internal(format!("cwd: {e}")))?;
            format!(
                "-{}",
                cwd.to_string_lossy().replace('/', "-").trim_start_matches('-')
            )
        }
    };
    Ok(PathBuf::from(base).join(&slug).join("memory"))
}

/// Parse frontmatter from a memory .md file: (name, type, description).
pub(crate) fn parse_frontmatter(content: &str) -> (Option<String>, Option<String>, Option<String>) {
    if !content.starts_with("---") {
        return (None, None, None);
    }
    let end = match content[3..].find("---") {
        Some(i) => i + 3,
        None => return (None, None, None),
    };
    let mut name = None;
    let mut mem_type = None;
    let mut desc = None;
    for line in content[3..end].lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("name:") {
            name = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("type:") {
            mem_type = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("description:") {
            desc = Some(v.trim().to_string());
        }
    }
    (name, mem_type, desc)
}

fn read_memory_entries(dir: &std::path::Path) -> Result<Vec<(String, String, PathBuf)>, ApiError> {
    let entries = fs::read_dir(dir)
        .map_err(|e| ApiError::internal(format!("read dir: {e}")))?;
    let mut result = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(true, |e| e != "md") {
            continue;
        }
        let fname = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        if fname == "MEMORY.md" {
            continue;
        }
        let content = fs::read_to_string(&path).unwrap_or_default();
        result.push((fname, content, path));
    }
    Ok(result)
}

async fn list_memories(Query(qs): Query<MemoryQuery>) -> Result<Json<Value>, ApiError> {
    let dir = resolve_memory_dir(qs.slug.as_deref())?;
    if !dir.exists() {
        return Ok(Json(json!({"ok": true, "memories": []})));
    }
    let mut memories = Vec::new();
    for (fname, content, path) in read_memory_entries(&dir)? {
        let (name, mem_type, description) = parse_frontmatter(&content);
        let meta = fs::metadata(&path).ok();
        let modified = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        memories.push(json!({
            "filename": fname,
            "name": name,
            "type": mem_type,
            "description": description,
            "size_bytes": meta.map(|m| m.len()).unwrap_or(0),
            "modified_at": modified,
        }));
    }
    Ok(Json(json!({"ok": true, "memories": memories})))
}

async fn memory_stats(Query(qs): Query<MemoryQuery>) -> Result<Json<Value>, ApiError> {
    let dir = resolve_memory_dir(qs.slug.as_deref())?;
    if !dir.exists() {
        return Ok(Json(json!({"ok": true, "total": 0, "estimated_tokens": 0, "by_type": {}})));
    }
    let mut total = 0u64;
    let mut total_words = 0u64;
    let mut by_type: HashMap<String, u64> = HashMap::new();
    for (_fname, content, _path) in read_memory_entries(&dir)? {
        let (_, mem_type, _) = parse_frontmatter(&content);
        total += 1;
        total_words += content.split_whitespace().count() as u64;
        *by_type
            .entry(mem_type.unwrap_or_else(|| "unknown".into()))
            .or_insert(0) += 1;
    }
    let estimated_tokens = (total_words as f64 * 1.5) as u64;
    Ok(Json(json!({
        "ok": true, "total": total,
        "estimated_tokens": estimated_tokens, "by_type": by_type,
    })))
}

async fn delete_memory(
    Path(filename): Path<String>,
    Query(qs): Query<MemoryQuery>,
) -> Result<Json<Value>, ApiError> {
    if filename.contains("..") || filename.contains('/') {
        return Err(ApiError::bad_request("invalid filename"));
    }
    let dir = resolve_memory_dir(qs.slug.as_deref())?;
    let path = dir.join(&filename);
    if !path.exists() {
        return Err(ApiError::not_found(format!("not found: {filename}")));
    }
    fs::remove_file(&path).map_err(|e| ApiError::internal(format!("delete: {e}")))?;
    Ok(Json(json!({"ok": true, "deleted": filename})))
}

async fn garbage_collect(Query(qs): Query<MemoryQuery>) -> Result<Json<Value>, ApiError> {
    let dir = resolve_memory_dir(qs.slug.as_deref())?;
    if !dir.exists() {
        return Ok(Json(json!({"ok": true, "deleted": [], "archived": [], "kept": []})));
    }
    let git_log = std::process::Command::new("git")
        .args(["log", "--oneline", "-100", "--all"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let now = std::time::SystemTime::now();
    let thirty_days = std::time::Duration::from_secs(30 * 24 * 3600);
    let (mut deleted, mut archived, mut kept) = (Vec::new(), Vec::new(), Vec::new());

    for (fname, content, path) in read_memory_entries(&dir)? {
        let (name, mem_type, _) = parse_frontmatter(&content);
        let is_old = fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok())
            .map_or(false, |m| {
                now.duration_since(m).map_or(false, |age| age > thirty_days)
            });
        let ref_name = name.as_deref().unwrap_or(&fname);
        let in_git = git_log.contains(ref_name);

        if mem_type.as_deref() == Some("feedback") && in_git {
            fs::remove_file(&path).ok();
            deleted.push(fname);
        } else if is_old && !in_git {
            let archive_dir = dir.join(".archived");
            fs::create_dir_all(&archive_dir).ok();
            if fs::rename(&path, archive_dir.join(&fname)).is_ok() {
                archived.push(fname);
            } else {
                kept.push(fname);
            }
        } else {
            kept.push(fname);
        }
    }
    Ok(Json(json!({"ok": true, "deleted": deleted, "archived": archived, "kept": kept})))
}
