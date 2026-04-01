/// Plan-Org linkage: org_id column on plans + query plans by org.
use super::state::{query_rows, ApiError, ServerState};
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

/// Idempotent migration: add org_id column to plans table.
/// Called once at router init time; ALTER TABLE errors are silently
/// ignored when the column already exists (SQLite behaviour).
pub fn migrate(state: &ServerState) -> Result<(), ApiError> {
    let conn = state.get_conn()?;
    let _ = conn.execute_batch("ALTER TABLE plans ADD COLUMN org_id TEXT;");
    let _ = conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_plans_org_id ON plans(org_id);",
    );
    Ok(())
}

/// GET /api/orgs/:slug/plans — list plans linked to an org.
async fn plans_by_org(
    State(state): State<ServerState>,
    Path(slug): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let plans = query_rows(
        &conn,
        "SELECT id, project_id, name, status, org_id, created_at \
         FROM plans WHERE org_id = ?1 ORDER BY id DESC",
        rusqlite::params![slug],
    )?;
    Ok(Json(json!({ "ok": true, "plans": plans })))
}

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/orgs/:slug/plans", get(plans_by_org))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::state::ServerState;
    use rusqlite::params;
    use std::path::PathBuf;

    fn test_state() -> ServerState {
        let dir = tempfile::tempdir().expect("tmpdir");
        let db_path = dir.path().join("test_plan_org.db");
        // Leak dir so it lives long enough for the test
        std::mem::forget(dir);
        ServerState::new(db_path, None)
    }

    #[test]
    fn test_migrate_idempotent() {
        let state = test_state();
        // Running migrate twice must not error
        migrate(&state).expect("first migrate");
        migrate(&state).expect("second migrate (idempotent)");
    }

    #[test]
    fn test_insert_plan_with_org_id() {
        let state = test_state();
        migrate(&state).expect("migrate");
        let conn = state.get_conn().expect("conn");
        conn.execute(
            "INSERT INTO plans (project_id, name, status, org_id) \
             VALUES (?1, ?2, ?3, ?4)",
            params!["proj-1", "Alpha rollout", "todo", "org-acme"],
        )
        .expect("insert");
        let rows = query_rows(
            &conn,
            "SELECT id, name, org_id FROM plans WHERE org_id = ?1",
            params!["org-acme"],
        )
        .expect("query");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["org_id"], "org-acme");
        assert_eq!(rows[0]["name"], "Alpha rollout");
    }

    #[test]
    fn test_query_by_org_filters_correctly() {
        let state = test_state();
        migrate(&state).expect("migrate");
        let conn = state.get_conn().expect("conn");
        conn.execute(
            "INSERT INTO plans (project_id, name, status, org_id) \
             VALUES (?1, ?2, ?3, ?4)",
            params!["proj-1", "Plan A", "todo", "org-alpha"],
        )
        .expect("insert A");
        conn.execute(
            "INSERT INTO plans (project_id, name, status, org_id) \
             VALUES (?1, ?2, ?3, ?4)",
            params!["proj-2", "Plan B", "active", "org-beta"],
        )
        .expect("insert B");
        conn.execute(
            "INSERT INTO plans (project_id, name, status, org_id) \
             VALUES (?1, ?2, ?3, ?4)",
            params!["proj-3", "Plan C", "done", "org-alpha"],
        )
        .expect("insert C");

        let alpha = query_rows(
            &conn,
            "SELECT id, name, org_id FROM plans WHERE org_id = ?1 ORDER BY id",
            params!["org-alpha"],
        )
        .expect("query alpha");
        assert_eq!(alpha.len(), 2);
        assert_eq!(alpha[0]["name"], "Plan A");
        assert_eq!(alpha[1]["name"], "Plan C");

        let beta = query_rows(
            &conn,
            "SELECT id, name, org_id FROM plans WHERE org_id = ?1",
            params!["org-beta"],
        )
        .expect("query beta");
        assert_eq!(beta.len(), 1);
        assert_eq!(beta[0]["name"], "Plan B");

        let empty = query_rows(
            &conn,
            "SELECT id FROM plans WHERE org_id = ?1",
            params!["org-none"],
        )
        .expect("query empty");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_plans_without_org_id_excluded() {
        let state = test_state();
        migrate(&state).expect("migrate");
        let conn = state.get_conn().expect("conn");
        conn.execute(
            "INSERT INTO plans (project_id, name, status) \
             VALUES (?1, ?2, ?3)",
            params!["proj-1", "Orphan plan", "todo"],
        )
        .expect("insert orphan");
        conn.execute(
            "INSERT INTO plans (project_id, name, status, org_id) \
             VALUES (?1, ?2, ?3, ?4)",
            params!["proj-2", "Linked plan", "todo", "org-gamma"],
        )
        .expect("insert linked");
        let rows = query_rows(
            &conn,
            "SELECT id, name FROM plans WHERE org_id = ?1",
            params!["org-gamma"],
        )
        .expect("query");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "Linked plan");
    }
}
