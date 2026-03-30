use super::state::{ApiError, ServerState};
use crate::memory::sqlite_store::SqliteMemoryStore;
use crate::memory::types::{AccessLevel, Attestation, Memory, MemoryType, RecallQuery};
use crate::memory::MemoryStore;
use axum::extract::{Path, Query, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/memory/remember", post(remember))
        .route("/api/memory/recall", get(recall))
        .route("/api/memory/forget/{id}", delete(forget))
        .route("/api/memory/share", post(share))
        .route("/api/memory/attest", post(attest))
}

// -- Request types --

#[derive(Deserialize)]
struct RememberBody {
    agent_id: String,
    memory_type: String,
    content: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default = "default_access")]
    access_level: String,
}

fn default_access() -> String {
    "Private".to_string()
}

#[derive(Deserialize)]
struct RecallParams {
    query: Option<String>,
    semantic: Option<String>,
    #[serde(rename = "type")]
    memory_type: Option<String>,
    agent: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
    fts_weight: Option<f32>,
}

fn default_limit() -> usize {
    100
}

#[derive(Deserialize)]
struct ShareBody {
    memory_id: String,
    target_agent_ids: Vec<String>,
}

#[derive(Deserialize)]
struct AttestBody {
    memory_id: String,
    attesting_agent_id: String,
    confidence: f64,
}

// -- Helpers --

/// Resolve the memory DB path: CONVERGIO_MEMORY_PATH env var, or $HOME/.claude/memory/memory.db.
/// Using an absolute path prevents 500 errors when the daemon runs from a non-repo cwd.
fn memory_db_path() -> String {
    if let Ok(p) = std::env::var("CONVERGIO_MEMORY_PATH") {
        if !p.is_empty() {
            return p;
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!("{home}/.claude/memory/memory.db")
}

fn open_store(_state: &ServerState) -> Result<SqliteMemoryStore, ApiError> {
    let path = memory_db_path();
    // Ensure parent directory exists so SQLite can create the file.
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ApiError::internal(format!("cannot create memory dir: {e}")))?;
    }
    SqliteMemoryStore::new(&path).map_err(|e| ApiError::internal(e.to_string()))
}

fn parse_memory_type(s: &str) -> Result<MemoryType, ApiError> {
    match s {
        "Fact" => Ok(MemoryType::Fact),
        "Decision" => Ok(MemoryType::Decision),
        "Preference" => Ok(MemoryType::Preference),
        "Observation" => Ok(MemoryType::Observation),
        other => Err(ApiError::bad_request(format!(
            "invalid memory_type: {other}"
        ))),
    }
}

fn parse_access_level(s: &str) -> AccessLevel {
    match s {
        "Shared" => AccessLevel::Shared,
        "Public" => AccessLevel::Public,
        _ => AccessLevel::Private,
    }
}

// -- Handlers --

async fn remember(
    State(state): State<ServerState>,
    Json(body): Json<RememberBody>,
) -> Result<Json<Value>, ApiError> {
    let store = open_store(&state)?;
    let mem = Memory {
        id: String::new(),
        agent_id: body.agent_id,
        memory_type: parse_memory_type(&body.memory_type)?,
        content: body.content,
        tags: body.tags,
        created_at: Utc::now(),
        expires_at: None,
        access_level: parse_access_level(&body.access_level),
        attestations: vec![],
    };
    let id = store
        .remember(mem)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(json!({"ok": true, "memory_id": id})))
}

async fn recall(
    State(state): State<ServerState>,
    Query(params): Query<RecallParams>,
) -> Result<Json<Value>, ApiError> {
    let store = open_store(&state)?;
    let mt = params
        .memory_type
        .as_deref()
        .map(parse_memory_type)
        .transpose()?;
    let query = RecallQuery {
        memory_type: mt,
        text_search: params.query,
        semantic_query: params.semantic,
        agent_id: params.agent,
        limit: params.limit,
        fts_weight: params.fts_weight.unwrap_or(0.5),
        ..Default::default()
    };
    let memories = store
        .recall(query)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let items: Vec<Value> = memories.iter().map(|m| json!(m)).collect();
    Ok(Json(json!({"ok": true, "memories": items})))
}

async fn forget(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let store = open_store(&state)?;
    store
        .forget(&id)
        .map_err(|e| ApiError::not_found(e.to_string()))?;
    Ok(Json(json!({"ok": true})))
}

async fn share(
    State(state): State<ServerState>,
    Json(body): Json<ShareBody>,
) -> Result<Json<Value>, ApiError> {
    let store = open_store(&state)?;
    store
        .share(&body.memory_id, &body.target_agent_ids)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(json!({"ok": true})))
}

async fn attest(
    State(state): State<ServerState>,
    Json(body): Json<AttestBody>,
) -> Result<Json<Value>, ApiError> {
    let store = open_store(&state)?;
    let attestation = Attestation {
        attesting_agent_id: body.attesting_agent_id,
        timestamp: Utc::now(),
        confidence: body.confidence,
    };
    store
        .attest(&body.memory_id, attestation)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(json!({"ok": true})))
}
