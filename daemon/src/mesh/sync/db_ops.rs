// Re-exports for backwards compatibility — actual code is in conn.rs and ops.rs.

pub use super::conn::{
    ensure_sync_schema, ensure_sync_schema_pub, now_ms, open_persistent_sync_conn, open_sync_conn,
    validate_peer_name,
};
pub use super::ops::{
    apply_delta_frame, collect_changes_since, collect_changes_with_conn, current_db_version,
    current_db_version_with_conn, read_changes_since_from_conn, record_sent_stats,
    record_sent_stats_with_conn, record_sync_error,
};
