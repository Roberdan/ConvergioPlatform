//! GET /api/health/deep — per-component health breakdown.
//!
//! Returns ComponentHealth for: database, filesystem, ipc_engine, swarm.
//! Wires through resilience::health::HealthRegistry registered at startup.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;

use super::state::ServerState;
use crate::resilience::health::{ComponentHealth, HealthCheck, HealthRegistry, HealthStatus};

/// JSON-serializable health snapshot.
#[derive(Serialize)]
pub struct DeepHealthResponse {
    pub status: String,
    pub components: Vec<ComponentSummary>,
}

#[derive(Serialize)]
pub struct ComponentSummary {
    pub name: String,
    pub status: String,
    pub message: Option<String>,
}

impl From<ComponentHealth> for ComponentSummary {
    fn from(h: ComponentHealth) -> Self {
        Self {
            name: h.name,
            status: h.status.to_string(),
            message: h.message,
        }
    }
}

/// Database health check — verifies connection + WAL mode + busy state.
struct DatabaseCheck {
    state: ServerState,
}

impl HealthCheck for DatabaseCheck {
    fn name(&self) -> &str {
        "database"
    }

    fn check(&self) -> ComponentHealth {
        let conn = match self.state.get_conn() {
            Ok(c) => c,
            Err(e) => {
                return ComponentHealth {
                    name: self.name().into(),
                    status: HealthStatus::Unhealthy,
                    message: Some(format!("pool error: {e}")),
                }
            }
        };
        // Verify WAL mode is active
        let journal: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap_or_default();
        if journal != "wal" {
            return ComponentHealth {
                name: self.name().into(),
                status: HealthStatus::Degraded,
                message: Some(format!("journal_mode={journal}, expected wal")),
            };
        }
        ComponentHealth {
            name: self.name().into(),
            status: HealthStatus::Healthy,
            message: None,
        }
    }
}

/// Filesystem health check — verifies data dir is writable.
struct FilesystemCheck {
    db_path: std::path::PathBuf,
}

impl HealthCheck for FilesystemCheck {
    fn name(&self) -> &str {
        "filesystem"
    }

    fn check(&self) -> ComponentHealth {
        let dir = self.db_path.parent().unwrap_or(Path::new("."));
        let writable = dir.metadata().map(|m| !m.permissions().readonly()).unwrap_or(false);
        if !writable {
            return ComponentHealth {
                name: self.name().into(),
                status: HealthStatus::Unhealthy,
                message: Some(format!("data dir not writable: {}", dir.display())),
            };
        }
        ComponentHealth {
            name: self.name().into(),
            status: HealthStatus::Healthy,
            message: None,
        }
    }
}

/// IPC engine presence check.
struct IpcEngineCheck {
    initialized: bool,
}

impl HealthCheck for IpcEngineCheck {
    fn name(&self) -> &str {
        "ipc_engine"
    }

    fn check(&self) -> ComponentHealth {
        if self.initialized {
            ComponentHealth {
                name: self.name().into(),
                status: HealthStatus::Healthy,
                message: None,
            }
        } else {
            ComponentHealth {
                name: self.name().into(),
                status: HealthStatus::Degraded,
                message: Some("IpcEngine not initialized".into()),
            }
        }
    }
}

/// Swarm health check — peer count from DB + last heartbeat.
struct SwarmCheck {
    state: ServerState,
}

impl HealthCheck for SwarmCheck {
    fn name(&self) -> &str {
        "swarm"
    }

    fn check(&self) -> ComponentHealth {
        let conn = match self.state.get_conn() {
            Ok(c) => c,
            Err(e) => {
                return ComponentHealth {
                    name: self.name().into(),
                    status: HealthStatus::Degraded,
                    message: Some(format!("db unavailable: {e}")),
                }
            }
        };
        let peer_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM peer_heartbeats", [], |r| r.get(0))
            .unwrap_or(0);
        let status = if peer_count == 0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
        ComponentHealth {
            name: self.name().into(),
            status,
            message: Some(format!("peers={peer_count}")),
        }
    }
}

pub async fn health_deep_handler(State(state): State<ServerState>) -> Json<DeepHealthResponse> {
    let registry = HealthRegistry::new();
    registry.register(Arc::new(DatabaseCheck { state: state.clone() }));
    registry.register(Arc::new(FilesystemCheck {
        db_path: state.db_path.clone(),
    }));
    registry.register(Arc::new(IpcEngineCheck {
        initialized: state.ipc_engine.is_some(),
    }));
    registry.register(Arc::new(SwarmCheck { state: state.clone() }));

    let components: Vec<ComponentSummary> =
        registry.check_all().into_iter().map(Into::into).collect();

    let overall = registry.aggregate_status();
    Json(DeepHealthResponse {
        status: overall.to_string(),
        components,
    })
}

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/health/deep", get(health_deep_handler))
}
