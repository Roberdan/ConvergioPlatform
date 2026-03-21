mod conn;
mod db_ops;
mod frame_io;
mod ops;
mod ops_apply;
mod types;

#[cfg(test)]
pub(super) use ops_apply::{apply_changes_to_conn, get_crr_table_allowlist};

#[path = "../sync_batch.rs"]
mod sync_batch;
#[cfg(test)]
#[path = "../sync_frame_tests.rs"]
mod sync_frame_tests;
#[cfg(test)]
#[path = "../sync_tests.rs"]
mod sync_tests;

pub use db_ops::{
    apply_delta_frame, collect_changes_since, collect_changes_with_conn, current_db_version,
    current_db_version_with_conn, ensure_sync_schema_pub, open_persistent_sync_conn,
    open_sync_conn, read_changes_since_from_conn, record_sent_stats, record_sent_stats_with_conn,
    record_sync_error,
};
pub use frame_io::{read_frame, read_frame_with_quota, write_frame};
pub use sync_batch::{current_time_ms, SyncBatchWindow};
pub use types::{
    ApplySummary, DeltaChange, FramedMeshSyncFrame, MeshSyncFrame, PeerQuota,
    MAX_FRAME_BYTES, MAX_PEER_NAME_LEN, MAX_PENDING_PEER_BYTES,
};
