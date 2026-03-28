use super::state::{ApiError, ServerState};
use crate::capabilities::checklist_integration::SecurityGate;
use crate::capabilities::permissions::PermissionManager;
use crate::capabilities::proxy::CapabilityProxy;
use crate::capabilities::registry::CapabilityRegistry;
use crate::capabilities::ring::Ring;
use crate::capabilities::types::Capability;
use axum::extract::{Path, Query};
use axum::routing::{get, post, put};
use axum::{Extension, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

/// Shared capability state injected via Extension.
pub struct CapState {
    pub registry: CapabilityRegistry,
    pub proxy: CapabilityProxy,
    pub permissions: PermissionManager,
}

impl CapState {
    pub fn new() -> Self {
        Self {
            registry: CapabilityRegistry::new(),
            proxy: CapabilityProxy::new(),
            permissions: PermissionManager::new(),
        }
    }
}

pub fn router() -> Router<ServerState> {
    let cap_state = Arc::new(CapState::new());
    Router::new()
        .route("/api/capabilities/list", get(list_capabilities))
        .route("/api/capabilities/invoke", post(invoke_capability))
        .route("/api/capabilities/register", post(register_capability))
        .route("/api/capabilities/schema/{name}", get(get_schema))
        .route("/api/capabilities/permissions", put(update_permissions))
        .layer(Extension(cap_state))
}

#[derive(Deserialize)]
struct ListParams {
    ring: Option<u8>,
}

async fn list_capabilities(
    Extension(cap): Extension<Arc<CapState>>,
    Query(params): Query<ListParams>,
) -> Result<Json<Value>, ApiError> {
    let ring_filter = params.ring.map(Ring::from_u8);
    let caps = cap.registry.list(ring_filter);
    let items: Vec<Value> = caps.iter().map(|c| json!(c)).collect();
    Ok(Json(json!({"ok": true, "capabilities": items, "count": items.len()})))
}

#[derive(Deserialize)]
struct InvokeBody {
    name: String,
    input: Value,
    agent_id: String,
}

async fn invoke_capability(
    Extension(cap): Extension<Arc<CapState>>,
    Json(body): Json<InvokeBody>,
) -> Result<Json<Value>, ApiError> {
    let capability = cap
        .registry
        .get(&body.name)
        .map_err(|e| ApiError::not_found(e.to_string()))?;

    // Permission check.
    cap.permissions
        .check(&body.agent_id, &body.name, capability.ring_level())
        .map_err(|e| ApiError::bad_request(e.to_string()))?;

    // Proxy check (rate limit + circuit breaker).
    cap.proxy
        .pre_invoke(&body.agent_id, &body.name)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;

    // For now, return the capability info (actual MCP invocation requires
    // an active MCP connector which is wired in the daemon serve loop).
    cap.proxy.record_success(&body.name);
    Ok(Json(json!({
        "ok": true,
        "capability": body.name,
        "ring": capability.ring,
        "input_received": body.input,
        "note": "MCP invocation requires running daemon with active connectors"
    })))
}

#[derive(Deserialize)]
struct RegisterBody {
    name: String,
    description: String,
    ring: u8,
    #[serde(default)]
    mcp_server: Option<String>,
    #[serde(default)]
    input_schema: Option<Value>,
    #[serde(default)]
    permissions_required: Vec<String>,
}

async fn register_capability(
    Extension(cap): Extension<Arc<CapState>>,
    Json(body): Json<RegisterBody>,
) -> Result<Json<Value>, ApiError> {
    let capability = Capability {
        name: body.name,
        description: body.description,
        ring: body.ring,
        mcp_server: body.mcp_server,
        input_schema: body.input_schema.unwrap_or(json!({})),
        permissions_required: body.permissions_required,
        enabled: true,
    };

    // Security gate validation.
    let gate = SecurityGate::validate_registration(&capability);
    if !gate.passed {
        let failures: Vec<String> = gate
            .checks
            .iter()
            .filter(|c| !c.passed)
            .map(|c| format!("{}: {}", c.name, c.detail))
            .collect();
        return Err(ApiError::bad_request(format!(
            "security gate failed: {}",
            failures.join("; ")
        )));
    }

    cap.registry
        .register(capability)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(json!({"ok": true})))
}

async fn get_schema(
    Extension(cap): Extension<Arc<CapState>>,
    Path(name): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let capability = cap
        .registry
        .get(&name)
        .map_err(|e| ApiError::not_found(e.to_string()))?;
    Ok(Json(json!({
        "ok": true,
        "name": capability.name,
        "description": capability.description,
        "ring": capability.ring,
        "input_schema": capability.input_schema,
        "permissions_required": capability.permissions_required,
    })))
}

#[derive(Deserialize)]
struct PermissionsBody {
    agent_id: String,
    grant: Option<String>,
    revoke: Option<String>,
}

async fn update_permissions(
    Extension(cap): Extension<Arc<CapState>>,
    Json(body): Json<PermissionsBody>,
) -> Result<Json<Value>, ApiError> {
    if let Some(tool) = body.grant {
        cap.permissions.grant(&body.agent_id, &tool);
    }
    if let Some(tool) = body.revoke {
        cap.permissions.revoke(&body.agent_id, &tool);
    }
    let perms = cap.permissions.get(&body.agent_id);
    Ok(Json(json!({
        "ok": true,
        "agent_id": body.agent_id,
        "max_ring": perms.max_ring.as_u8(),
        "allowed_tools": perms.allowed_tools.iter().collect::<Vec<_>>(),
    })))
}
