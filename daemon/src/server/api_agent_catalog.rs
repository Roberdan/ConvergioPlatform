// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Agent catalog CRUD: sync from .agent.md files, enable/disable, create, list.

use super::state::{query_rows, ApiError, ServerState};
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use rusqlite::params;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/agents/catalog", get(catalog_list))
        .route("/api/agents/sync", post(catalog_sync))
        .route("/api/agents/enable", post(catalog_enable))
        .route("/api/agents/disable", post(catalog_disable))
        .route("/api/agents/create", post(catalog_create))
}

// -- Request bodies --

#[derive(Deserialize)]
struct SyncBody {
    source_dir: String,
}

#[derive(Deserialize)]
struct EnableBody {
    name: String,
    target_dir: String,
}

#[derive(Deserialize)]
struct DisableBody {
    name: String,
    target_dir: String,
}

#[derive(Deserialize)]
struct CreateBody {
    name: String,
    category: Option<String>,
    description: Option<String>,
    model: Option<String>,
    tools: Option<String>,
}

#[derive(Deserialize)]
struct CatalogQuery {
    category: Option<String>,
}

// -- Handlers --

async fn catalog_list(
    State(state): State<ServerState>,
    Query(q): Query<CatalogQuery>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let agents = if let Some(cat) = &q.category {
        query_rows(
            &conn,
            "SELECT * FROM agent_catalog WHERE category = ?1 ORDER BY name",
            params![cat],
        )?
    } else {
        query_rows(&conn, "SELECT * FROM agent_catalog ORDER BY name", [])?
    };
    Ok(Json(json!({"ok": true, "agents": agents})))
}

async fn catalog_sync(
    State(state): State<ServerState>,
    Json(body): Json<SyncBody>,
) -> Result<Json<Value>, ApiError> {
    let dir = body.source_dir.trim();
    if dir.is_empty() {
        return Err(ApiError::bad_request("source_dir is required"));
    }
    let path = Path::new(dir);
    if !path.is_dir() {
        return Err(ApiError::bad_request(format!(
            "source_dir not found: {dir}"
        )));
    }

    let entries = fs::read_dir(path)
        .map_err(|e| ApiError::internal(format!("read_dir failed: {e}")))?;

    let conn = state.get_conn()?;
    let mut synced = 0u32;
    let mut added = 0u32;

    for entry in entries.flatten() {
        let fname = entry.file_name();
        let fname_str = fname.to_string_lossy();
        if !fname_str.ends_with(".agent.md") {
            continue;
        }
        let content = fs::read_to_string(entry.path())
            .map_err(|e| ApiError::internal(format!("read failed: {e}")))?;

        let Some(front) = parse_yaml_frontmatter(&content) else {
            continue;
        };
        let name = front.name.unwrap_or_default();
        if name.is_empty() {
            continue;
        }

        // Check if row already exists
        let exists: bool = conn
            .prepare("SELECT 1 FROM agent_catalog WHERE name = ?1")
            .and_then(|mut s| s.exists(params![&name]))
            .unwrap_or(false);

        conn.execute(
            "INSERT OR REPLACE INTO agent_catalog \
             (name, description, model, tools, source_repo, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
            params![
                &name,
                &front.description.unwrap_or_default(),
                &front.model.unwrap_or_default(),
                &front.tools.unwrap_or_default(),
                dir,
            ],
        )
        .map_err(|e| ApiError::internal(format!("upsert failed: {e}")))?;

        if !exists {
            added += 1;
        }
        synced += 1;
    }

    Ok(Json(json!({"ok": true, "synced": synced, "added": added})))
}

async fn catalog_enable(
    State(state): State<ServerState>,
    Json(body): Json<EnableBody>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let row = query_rows(
        &conn,
        "SELECT * FROM agent_catalog WHERE name = ?1",
        params![&body.name],
    )?;
    let agent = row
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::bad_request(format!("agent not found: {}", body.name)))?;

    let target = Path::new(&body.target_dir);
    if !target.is_dir() {
        return Err(ApiError::bad_request("target_dir does not exist"));
    }

    let file_path = target.join(format!("{}.agent.md", body.name));
    let description = agent["description"].as_str().unwrap_or("");
    let model = agent["model"].as_str().unwrap_or("claude-sonnet-4-6");
    let tools_str = agent["tools"].as_str().unwrap_or("view, edit, bash");

    let content = format!(
        "---\nname: {}\ndescription: \"{}\"\nmodel: {}\ntools:\n{}\n---\n\n# {}\n\n{}\n",
        body.name,
        description,
        model,
        tools_str
            .split(',')
            .map(|t| format!("  - {}", t.trim()))
            .collect::<Vec<_>>()
            .join("\n"),
        body.name,
        description,
    );

    fs::write(&file_path, content)
        .map_err(|e| ApiError::internal(format!("write failed: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "enabled": body.name,
        "path": file_path.to_string_lossy(),
    })))
}

async fn catalog_disable(
    State(_state): State<ServerState>,
    Json(body): Json<DisableBody>,
) -> Result<Json<Value>, ApiError> {
    let file_path = Path::new(&body.target_dir).join(format!("{}.agent.md", body.name));
    if file_path.exists() {
        fs::remove_file(&file_path)
            .map_err(|e| ApiError::internal(format!("remove failed: {e}")))?;
    }
    Ok(Json(json!({"ok": true, "disabled": body.name})))
}

async fn catalog_create(
    State(state): State<ServerState>,
    Json(body): Json<CreateBody>,
) -> Result<Json<Value>, ApiError> {
    if body.name.trim().is_empty() {
        return Err(ApiError::bad_request("name is required"));
    }
    let conn = state.get_conn()?;
    conn.execute(
        "INSERT INTO agent_catalog (name, category, description, model, tools) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            body.name.trim(),
            body.category.as_deref().unwrap_or(""),
            body.description.as_deref().unwrap_or(""),
            body.model.as_deref().unwrap_or(""),
            body.tools.as_deref().unwrap_or(""),
        ],
    )
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("UNIQUE") {
            ApiError::conflict(format!("agent already exists: {}", body.name))
        } else {
            ApiError::internal(format!("insert failed: {e}"))
        }
    })?;
    Ok(Json(json!({"ok": true, "created": body.name.trim()})))
}

// -- YAML frontmatter parser --

struct Frontmatter {
    name: Option<String>,
    description: Option<String>,
    model: Option<String>,
    tools: Option<String>,
}

fn parse_yaml_frontmatter(content: &str) -> Option<Frontmatter> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after_first = &trimmed[3..];
    let end = after_first.find("\n---")?;
    let yaml_block = &after_first[..end];

    let mut name = None;
    let mut description = None;
    let mut model = None;
    let mut tools_list: Vec<String> = Vec::new();
    let mut in_tools = false;

    for line in yaml_block.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.starts_with("name:") {
            name = Some(extract_value(trimmed_line, "name:"));
            in_tools = false;
        } else if trimmed_line.starts_with("description:") {
            description = Some(extract_value(trimmed_line, "description:"));
            in_tools = false;
        } else if trimmed_line.starts_with("model:") {
            model = Some(extract_value(trimmed_line, "model:"));
            in_tools = false;
        } else if trimmed_line.starts_with("tools:") {
            in_tools = true;
        } else if in_tools && trimmed_line.starts_with("- ") {
            tools_list.push(trimmed_line[2..].trim().to_string());
        } else if !trimmed_line.starts_with('-') && trimmed_line.contains(':') {
            in_tools = false;
        }
    }

    let tools = if tools_list.is_empty() {
        None
    } else {
        Some(tools_list.join(", "))
    };

    Some(Frontmatter {
        name,
        description,
        model,
        tools,
    })
}

fn extract_value(line: &str, prefix: &str) -> String {
    line[prefix.len()..]
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}
