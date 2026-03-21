// Dedicated DB thread for mesh sync — owns a persistent SQLite connection.
// Separated from daemon_sync_loops.rs to keep each file under 250 lines.

use crate::mesh::sync;
use super::DaemonConfig;

// Type alias to simplify complex channel types used below
type SyncReply = std::sync::mpsc::Sender<Result<(Vec<sync::DeltaChange>, i64, i64), String>>;

/// Messages for the dedicated DB sync thread
pub(super) enum SyncDbCmd {
    CollectChanges {
        cursor: i64,
        reply: SyncReply,
    },
    RecordSent {
        peer: String,
        count: usize,
        version: i64,
    },
    /// T1-01: Anti-entropy — get peer's last known db_version for catch-up
    GetPeerCursor {
        peer: String,
        reply: std::sync::mpsc::Sender<i64>,
    },
}

/// Spawn a single DB thread that owns one persistent connection.
pub(super) fn spawn_sync_db_thread(
    config: &DaemonConfig,
) -> std::sync::mpsc::Sender<SyncDbCmd> {
    let (tx, rx) = std::sync::mpsc::channel::<SyncDbCmd>();
    let db_path = config.db_path.clone();
    let crsql_path = config.crsqlite_path.clone();
    std::thread::Builder::new()
        .name("mesh-sync-db".into())
        .spawn(move || {
            let conn = match sync::open_persistent_sync_conn(&db_path, crsql_path.as_deref()) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("mesh-sync-db: failed to open DB: {e}");
                    return;
                }
            };
            let _ = sync::ensure_sync_schema_pub(&conn);
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    SyncDbCmd::CollectChanges { cursor, reply } => {
                        let init_cursor = if cursor < 0 {
                            sync::current_db_version_with_conn(&conn).unwrap_or(0)
                        } else {
                            cursor
                        };
                        let result = sync::collect_changes_with_conn(&conn, init_cursor)
                            .map(|(changes, checkpoint)| (changes, checkpoint, init_cursor));
                        let _ = reply.send(result);
                    }
                    SyncDbCmd::RecordSent { peer, count, version } => {
                        let _ =
                            sync::record_sent_stats_with_conn(&conn, &peer, count, version);
                    }
                    SyncDbCmd::GetPeerCursor { peer, reply } => {
                        // T1-01: Anti-entropy — resume from peer's last known version
                        let cursor = conn
                            .query_row(
                                "SELECT COALESCE(last_db_version, 0) FROM mesh_sync_stats \
                                 WHERE peer_name = ?1",
                                rusqlite::params![peer],
                                |r| r.get::<_, i64>(0),
                            )
                            .unwrap_or(0);
                        let _ = reply.send(cursor);
                    }
                }
            }
        })
        .expect("spawn mesh-sync-db thread");
    tx
}
