use super::state_init::init_db_and_pool;
use crate::db::PlanDb;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, Params, Row};
use serde_json::{json, Map, Value};
use std::path::PathBuf;
use tokio::sync::broadcast;

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

    pub fn open_db(&self) -> Result<PlanDb, ApiError> {
        // Always use plain SQLite — CRSQLite extension loads per-connection
        // and leaks file descriptors (WAL handles not released on drop).
        // No API query uses CRDT functions; CRSQLite is only needed for sync.
        PlanDb::open_sqlite_path(&self.db_path)
            .map_err(|err| ApiError::internal(format!("db open failed: {err}")))
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

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({"ok": false, "error": self.message})),
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
