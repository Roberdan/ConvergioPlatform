// Service marketplace: list active services, query service requests.

use super::state::{query_rows, ApiError, ServerState};
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/services/marketplace", get(handle_marketplace))
        .route("/api/services/requests", get(handle_requests))
}

/// GET /api/services/marketplace — list all active services.
#[tracing::instrument(skip_all)]
async fn handle_marketplace(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let rows = query_rows(
        &conn,
        "SELECT id, org_id, name, endpoint, status, metadata \
         FROM ipc_org_services WHERE status = 'active' \
         ORDER BY name",
        [],
    )
    .map_err(|e| ApiError::internal(format!("marketplace query failed: {e}")))?;
    Ok(Json(json!({ "services": rows })))
}

#[derive(Debug, Deserialize)]
pub struct RequestsQuery {
    pub org: Option<String>,
    pub limit: Option<i64>,
}

/// GET /api/services/requests?org=<slug> — requests for an org.
#[tracing::instrument(skip_all)]
async fn handle_requests(
    State(state): State<ServerState>,
    Query(params): Query<RequestsQuery>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let limit = params.limit.unwrap_or(50).min(200);

    let (sql, values): (String, Vec<Box<dyn rusqlite::ToSql>>) =
        if let Some(ref org) = params.org {
            (
                format!(
                    "SELECT id, requester_org, provider_org, service_name, \
                     status, cost, created_at, completed_at \
                     FROM ipc_service_requests \
                     WHERE requester_org = ?1 OR provider_org = ?1 \
                     ORDER BY created_at DESC LIMIT {limit}"
                ),
                vec![Box::new(org.clone())],
            )
        } else {
            (
                format!(
                    "SELECT id, requester_org, provider_org, service_name, \
                     status, cost, created_at, completed_at \
                     FROM ipc_service_requests \
                     ORDER BY created_at DESC LIMIT {limit}"
                ),
                vec![],
            )
        };

    let rows = query_rows(
        &conn,
        &sql,
        rusqlite::params_from_iter(values.iter().map(|v| v.as_ref())),
    )
    .map_err(|e| ApiError::internal(format!("service requests query failed: {e}")))?;
    Ok(Json(json!({ "requests": rows })))
}

#[cfg(test)]
mod tests {
    use crate::db::PlanDb;
    use crate::server::state::query_rows;

    fn setup_db() -> PlanDb {
        let db = PlanDb::open_in_memory().expect("db");
        db.connection()
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS ipc_orgs (
                     id TEXT PRIMARY KEY NOT NULL,
                     mission TEXT NOT NULL,
                     objectives TEXT NOT NULL,
                     ceo_agent TEXT NOT NULL,
                     budget REAL NOT NULL DEFAULT 0,
                     status TEXT NOT NULL DEFAULT 'active',
                     created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                     updated_at TEXT DEFAULT CURRENT_TIMESTAMP
                 );
                 CREATE TABLE IF NOT EXISTS ipc_org_services (
                     id TEXT PRIMARY KEY NOT NULL,
                     org_id TEXT NOT NULL,
                     name TEXT NOT NULL,
                     endpoint TEXT NOT NULL,
                     status TEXT NOT NULL DEFAULT 'active',
                     metadata TEXT,
                     registered_at TEXT DEFAULT CURRENT_TIMESTAMP,
                     UNIQUE(org_id, name)
                 );
                 CREATE TABLE IF NOT EXISTS ipc_service_requests (
                     id TEXT PRIMARY KEY NOT NULL,
                     requester_org TEXT NOT NULL,
                     provider_org TEXT NOT NULL,
                     service_name TEXT NOT NULL,
                     status TEXT NOT NULL DEFAULT 'pending',
                     cost REAL,
                     request_payload TEXT,
                     response_payload TEXT,
                     created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                     completed_at TEXT
                 );",
            )
            .expect("schema");
        db
    }

    fn seed(db: &PlanDb) {
        let c = db.connection();
        c.execute_batch(
            "INSERT INTO ipc_orgs (id, mission, objectives, ceo_agent) \
             VALUES ('eng', 'Build software', 'Ship fast', 'ceo-eng');
             INSERT INTO ipc_orgs (id, mission, objectives, ceo_agent) \
             VALUES ('ops', 'Run infra', 'Uptime 99.9', 'ceo-ops');
             INSERT INTO ipc_org_services (id, org_id, name, endpoint, status) \
             VALUES ('svc-1', 'eng', 'code-review', '/api/review', 'active');
             INSERT INTO ipc_org_services (id, org_id, name, endpoint, status) \
             VALUES ('svc-2', 'ops', 'deploy', '/api/deploy', 'active');
             INSERT INTO ipc_org_services (id, org_id, name, endpoint, status) \
             VALUES ('svc-3', 'ops', 'monitor', '/api/monitor', 'inactive');
             INSERT INTO ipc_service_requests \
             (id, requester_org, provider_org, service_name, status, cost) \
             VALUES ('req-1', 'eng', 'ops', 'deploy', 'completed', 10.0);
             INSERT INTO ipc_service_requests \
             (id, requester_org, provider_org, service_name, status) \
             VALUES ('req-2', 'ops', 'eng', 'code-review', 'pending');",
        )
        .expect("seed");
    }

    #[test]
    fn marketplace_lists_active_services_only() {
        let db = setup_db();
        seed(&db);
        let conn = db.connection();
        let rows = query_rows(
            conn,
            "SELECT id, org_id, name, endpoint, status, metadata \
             FROM ipc_org_services WHERE status = 'active' ORDER BY name",
            [],
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        let names: Vec<&str> = rows.iter()
            .filter_map(|r| r.get("name").and_then(|v| v.as_str()))
            .collect();
        assert!(names.contains(&"code-review"));
        assert!(names.contains(&"deploy"));
        assert!(!names.contains(&"monitor"));
    }

    #[test]
    fn requests_filtered_by_org() {
        let db = setup_db();
        seed(&db);
        let conn = db.connection();
        let rows = query_rows(
            conn,
            "SELECT id FROM ipc_service_requests \
             WHERE requester_org = ?1 OR provider_org = ?1",
            rusqlite::params!["eng"],
        )
        .unwrap();
        assert_eq!(rows.len(), 2, "eng is requester on req-1, provider on req-2");
    }

    #[test]
    fn requests_unfiltered_returns_all() {
        let db = setup_db();
        seed(&db);
        let conn = db.connection();
        let rows = query_rows(
            conn,
            "SELECT id FROM ipc_service_requests ORDER BY created_at DESC",
            [],
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn requests_cost_nullable() {
        let db = setup_db();
        seed(&db);
        let conn = db.connection();
        let rows = query_rows(
            conn,
            "SELECT cost FROM ipc_service_requests WHERE id = 'req-2'",
            [],
        )
        .unwrap();
        assert!(rows[0].get("cost").map_or(true, |v| v.is_null()));
    }
}
