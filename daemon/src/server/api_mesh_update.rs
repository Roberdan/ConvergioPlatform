// GET /api/mesh/update-status — compare local version against mesh peers.

use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct UpdateStatus {
    update_available: bool,
    latest_version: String,
    current_version: String,
    peer_with_latest: String,
    rustc_mismatch: bool,
}

/// Compare semver-like version strings: "1.2.3" > "1.2.2".
/// Returns std::cmp::Ordering.
fn cmp_version(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let va = parse(a);
    let vb = parse(b);
    va.cmp(&vb)
}

pub(crate) async fn handle_update_status(
    State(state): State<ServerState>,
) -> Result<Json<UpdateStatus>, ApiError> {
    let conn = state.get_conn()?;
    let current = env!("CARGO_PKG_VERSION");

    let mut stmt = conn
        .prepare(
            "SELECT peer_name, version, rustc_version FROM peer_heartbeats \
             WHERE version IS NOT NULL",
        )
        .map_err(|e| ApiError::internal(format!("prepare: {e}")))?;

    let mut latest_version = current.to_owned();
    let mut peer_with_latest = String::new();
    let mut rustc_mismatch = false;
    let local_rustc = local_rustc_version();

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| ApiError::internal(format!("query: {e}")))?;

    for row in rows {
        let (peer, ver, rustc) = row.map_err(|e| ApiError::internal(format!("row: {e}")))?;
        if cmp_version(&ver, &latest_version) == std::cmp::Ordering::Greater {
            latest_version = ver;
            peer_with_latest = peer;
        }
        if let Some(rv) = rustc {
            if rv != local_rustc {
                rustc_mismatch = true;
            }
        }
    }

    let update_available = cmp_version(&latest_version, current) == std::cmp::Ordering::Greater;

    Ok(Json(UpdateStatus {
        update_available,
        latest_version,
        current_version: current.to_owned(),
        peer_with_latest,
        rustc_mismatch,
    }))
}

fn local_rustc_version() -> String {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|| "unknown".to_owned())
}
