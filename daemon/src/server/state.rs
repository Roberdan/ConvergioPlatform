use super::state_init::init_db_and_pool;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, Params, Row};
use serde_json::{json, Map, Value};
use std::path::PathBuf;
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct ServerState {
    pub db_path: PathBuf,
    pub crsqlite_path: Option<String>,
    pool: r2d2::Pool<SqliteConnectionManager>,
    pub ws_tx: broadcast::Sender<Value>,
    pub started_at: std::time::Instant,
}

impl ServerState {
    pub fn new(db_path: PathBuf, crsqlite_path: Option<String>) -> Self {
        let pool = init_db_and_pool(&db_path, &crsqlite_path);
        let (ws_tx, _) = broadcast::channel(256);
        Self {
            db_path,
            crsqlite_path,
            pool,
            ws_tx,
            started_at: std::time::Instant::now(),
        }
    }

    pub fn get_conn(&self) -> Result<PooledConnection<SqliteConnectionManager>, ApiError> {
        self.pool
            .get()
            .map_err(|err| ApiError::internal(format!("pool exhausted: {err}")))
    }

    /// Expose the connection pool for modules that need a Pool directly (e.g. WorkspaceManager).
    pub fn pool(&self) -> r2d2::Pool<SqliteConnectionManager> {
        self.pool.clone()
    }
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: message.into(),
        }
    }

    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.status.as_u16(), self.message)
    }
}

impl From<String> for ApiError {
    fn from(message: String) -> Self {
        Self::internal(message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let request_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().to_rfc3339();
        (
            self.status,
            Json(json!({
                "ok": false,
                "error": {
                    "code": self.status.as_u16(),
                    "message": self.message,
                    "request_id": request_id,
                    "timestamp": timestamp,
                }
            })),
        )
            .into_response()
    }
}

pub fn query_rows<P: Params>(
    conn: &Connection,
    sql: &str,
    params: P,
) -> Result<Vec<Value>, ApiError> {
    let mut stmt = conn
        .prepare(sql)
        .map_err(|err| ApiError::internal(format!("prepare failed: {err}")))?;
    let rows = stmt
        .query_map(params, row_to_json)
        .map_err(|err| ApiError::internal(format!("query failed: {err}")))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|err| ApiError::internal(format!("row decode failed: {err}")))
}

pub fn query_one<P: Params>(
    conn: &Connection,
    sql: &str,
    params: P,
) -> Result<Option<Value>, ApiError> {
    Ok(query_rows(conn, sql, params)?.into_iter().next())
}

fn row_to_json(row: &Row<'_>) -> rusqlite::Result<Value> {
    let mut object = Map::new();
    for (idx, column) in row.as_ref().column_names().iter().enumerate() {
        let value = row.get_ref(idx)?;
        let json_value = match value {
            ValueRef::Null => Value::Null,
            ValueRef::Integer(v) => Value::from(v),
            ValueRef::Real(v) => Value::from(v),
            ValueRef::Text(v) => Value::from(String::from_utf8_lossy(v).to_string()),
            ValueRef::Blob(v) => Value::from(format!("blob:{}", v.len())),
        };
        object.insert((*column).to_string(), json_value);
    }
    Ok(Value::Object(object))
}
