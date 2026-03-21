use crate::mesh::sync;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

use super::{now_ts, DaemonConfig};

use sync::MeshSyncFrame;

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

/// Spawn a single DB thread that owns one persistent connection
pub(super) fn spawn_sync_db_thread(config: &DaemonConfig) -> std::sync::mpsc::Sender<SyncDbCmd> {
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
                        let _ = sync::record_sent_stats_with_conn(&conn, &peer, count, version);
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

pub(super) fn spawn_heartbeat_loop(out_tx: mpsc::Sender<MeshSyncFrame>, node_id: String) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(5));
        loop {
            ticker.tick().await;
            if out_tx
                .send(MeshSyncFrame::Heartbeat {
                    node: node_id.clone(),
                    ts: now_ts(),
                })
                .await
                .is_err()
            {
                break;
            }
        }
    });
}

pub(super) fn spawn_delta_loop(
    out_tx: mpsc::Sender<MeshSyncFrame>,
    sync_peer: Arc<RwLock<String>>,
    node_id: String,
    config: DaemonConfig,
) {
    const MAX_STAGED: usize = 50_000;
    let db_tx = spawn_sync_db_thread(&config);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(2));
        let mut db_cursor: i64 = -1; // -1 = needs initialization from anti-entropy
        let mut batch_window = sync::SyncBatchWindow::new(50);
        let mut staged_changes = Vec::new();
        let mut idle_ticks: u32 = 0;
        let mut anti_entropy_done = false;
        loop {
            ticker.tick().await;
            if idle_ticks > 0 {
                let extra_wait = Duration::from_secs((2u64.pow(idle_ticks.min(4))).min(30));
                tokio::time::sleep(extra_wait).await;
            }
            let peer_name = sync_peer.read().await.clone();
            // T1-01: Anti-entropy — on first tick, get peer's last known cursor
            if !anti_entropy_done && db_cursor < 0 {
                let (reply_tx, reply_rx) = std::sync::mpsc::channel();
                if db_tx
                    .send(SyncDbCmd::GetPeerCursor {
                        peer: peer_name.clone(),
                        reply: reply_tx,
                    })
                    .is_ok()
                {
                    if let Ok(Ok(cursor)) =
                        tokio::task::spawn_blocking(move || reply_rx.recv()).await
                    {
                        if cursor > 0 {
                            db_cursor = cursor;
                        }
                    }
                }
                anti_entropy_done = true;
            }
            // Send collect command to DB thread
            let (reply_tx, reply_rx) = std::sync::mpsc::channel();
            if db_tx
                .send(SyncDbCmd::CollectChanges {
                    cursor: db_cursor,
                    reply: reply_tx,
                })
                .is_err()
            {
                break; // DB thread died
            }
            // Wait for reply (blocking but the DB work is fast)
            let db_result = tokio::task::spawn_blocking(move || reply_rx.recv())
                .await
                .ok()
                .and_then(|r| r.ok())
                .unwrap_or(Err("DB thread unavailable".into()));
            match db_result {
                Ok((changes, checkpoint, effective_cursor)) => {
                    if db_cursor < 0 {
                        db_cursor = effective_cursor;
                    }
                    if !changes.is_empty() {
                        db_cursor = checkpoint;
                        try_flush_if_staged_full(
                            &out_tx,
                            &db_tx,
                            &peer_name,
                            &node_id,
                            &mut staged_changes,
                            &mut batch_window,
                            MAX_STAGED,
                        )
                        .await
                        .unwrap_or(());
                        if changes.len() > MAX_STAGED {
                            eprintln!(
                                "WARN: incoming change batch exceeded {MAX_STAGED}, sending immediately"
                            );
                            let send_count = changes.len();
                            let frame = MeshSyncFrame::Delta {
                                node: node_id.clone(),
                                sent_at_ms: sync::current_time_ms(),
                                last_db_version: checkpoint,
                                changes,
                            };
                            if out_tx.send(frame).await.is_ok() {
                                let _ = db_tx.send(SyncDbCmd::RecordSent {
                                    peer: peer_name.clone(),
                                    count: send_count,
                                    version: checkpoint,
                                });
                                batch_window.clear();
                            } else {
                                break;
                            }
                        } else {
                            batch_window.observe_change(checkpoint);
                            staged_changes.extend(changes);
                        }
                        idle_ticks = 0;
                    } else {
                        idle_ticks = idle_ticks.saturating_add(1);
                    }
                    if !staged_changes.is_empty()
                        && batch_window.should_flush(sync::current_time_ms())
                    {
                        let send_count = staged_changes.len(); // T1-03: capture count BEFORE take
                        let last_db_version = batch_window.take_checkpoint();
                        let frame = MeshSyncFrame::Delta {
                            node: node_id.clone(),
                            sent_at_ms: sync::current_time_ms(),
                            last_db_version,
                            changes: std::mem::take(&mut staged_changes),
                        };
                        if out_tx.send(frame).await.is_ok() {
                            let _ = db_tx.send(SyncDbCmd::RecordSent {
                                peer: peer_name,
                                count: send_count,
                                version: last_db_version,
                            });
                            batch_window.clear();
                        } else {
                            break;
                        }
                    }
                }
                Err(_) => {
                    idle_ticks = idle_ticks.saturating_add(1);
                }
            }
        }
    });
}

/// Force-flush staged changes when buffer is nearing capacity
async fn try_flush_if_staged_full(
    out_tx: &mpsc::Sender<MeshSyncFrame>,
    db_tx: &std::sync::mpsc::Sender<SyncDbCmd>,
    peer_name: &str,
    node_id: &str,
    staged: &mut Vec<sync::DeltaChange>,
    window: &mut sync::SyncBatchWindow,
    max_staged: usize,
) -> Result<(), ()> {
    if staged.len() < max_staged || staged.is_empty() {
        return Ok(());
    }
    eprintln!("WARN: staged_changes exceeded {max_staged}, forcing flush");
    let send_count = staged.len();
    let last_db_version = window.take_checkpoint();
    let frame = MeshSyncFrame::Delta {
        node: node_id.to_string(),
        sent_at_ms: sync::current_time_ms(),
        last_db_version,
        changes: std::mem::take(staged),
    };
    if out_tx.send(frame).await.is_ok() {
        let _ = db_tx.send(SyncDbCmd::RecordSent {
            peer: peer_name.to_string(),
            count: send_count,
            version: last_db_version,
        });
        window.clear();
        Ok(())
    } else {
        Err(())
    }
}
