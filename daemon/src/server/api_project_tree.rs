// GET /api/project/:id/tree — hierarchical plan tree for a project.

use super::state::{ApiError, ServerState};
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::Value;

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/project/:id/tree", get(handle_project_tree))
}

async fn handle_project_tree(
    State(state): State<ServerState>,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let tree = crate::db::plan_hierarchy::project_plan_tree(&conn, &project_id)
        .map_err(|e| ApiError::internal(format!("plan tree query failed: {e}")))?;
    let json = serde_json::to_value(tree)
        .map_err(|e| ApiError::internal(format!("serialization failed: {e}")))?;
    Ok(Json(json))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tempfile::TempDir;
    use tower::ServiceExt;

    fn test_state() -> (ServerState, TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test.db");
        let state = ServerState::new(db_path, None);
        // Seed test data via the state's connection
        let conn = state.get_conn().unwrap();
        conn.execute_batch(
            "INSERT OR IGNORE INTO projects (id, name) VALUES ('p1', 'TestProject');
             INSERT INTO plans (project_id, name, status, tasks_done, tasks_total, is_master, execution_mode)
                 VALUES ('p1', 'Master', 'doing', 0, 0, 1, 'mixed');
             INSERT INTO plans (project_id, name, status, tasks_done, tasks_total, is_master, parent_plan_id)
                 VALUES ('p1', 'Child A', 'done', 5, 5, 0, 1);",
        )
        .unwrap();
        (state, tmp)
    }

    async fn body_json(resp: axum::response::Response) -> Value {
        let bytes = axum::body::to_bytes(resp.into_body(), 1_000_000)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn project_tree_returns_hierarchy() {
        let (state, _tmp) = test_state();
        let app = router().with_state(state);
        let req = Request::builder()
            .uri("/api/project/p1/tree")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["project_name"], "TestProject");
        assert_eq!(json["plans"].as_array().unwrap().len(), 1); // master only
        assert_eq!(json["plans"][0]["children"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn project_tree_unknown_project_returns_empty() {
        let (state, _tmp) = test_state();
        let app = router().with_state(state);
        let req = Request::builder()
            .uri("/api/project/unknown/tree")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["plans"].as_array().unwrap().len(), 0);
    }
}
