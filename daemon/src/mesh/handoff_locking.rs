// Handoff locking: delegation lock acquire/release and plan status merge.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DelegationLock {
    peer: String,
    ts: u64,
    pid: u32,
}

pub fn acquire_lock(
    lock_dir: &Path,
    plan_id: i64,
    peer: &str,
    ttl_secs: u64,
) -> Result<(), String> {
    fs::create_dir_all(lock_dir).map_err(|e| e.to_string())?;
    let lock_path = lock_dir.join(format!("delegate-{plan_id}.lock"));
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    if let Ok(raw) = fs::read_to_string(&lock_path) {
        if let Ok(existing) = serde_json::from_str::<DelegationLock>(&raw) {
            if now.saturating_sub(existing.ts) < ttl_secs {
                return Err(format!(
                    "locked by {} {}s ago",
                    existing.peer,
                    now.saturating_sub(existing.ts)
                ));
            }
        }
    }
    let payload = DelegationLock {
        peer: peer.to_string(),
        ts: now,
        pid: std::process::id(),
    };
    fs::write(
        lock_path,
        serde_json::to_string(&payload).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

pub fn release_lock(lock_dir: &Path, plan_id: i64) -> Result<(), String> {
    let lock_path = lock_dir.join(format!("delegate-{plan_id}.lock"));
    if lock_path.exists() {
        fs::remove_file(lock_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn merge_plan_status(plan_id: i64, local_db: &Path, remote_db: &Path) -> Result<usize, String> {
    let updates = {
        let remote = Connection::open(remote_db).map_err(|e| e.to_string())?;
        let local = Connection::open(local_db).map_err(|e| e.to_string())?;
        local
            .execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| e.to_string())?;
        let rank = HashMap::from([
            ("pending", 0),
            ("in_progress", 1),
            ("blocked", 1),
            ("submitted", 2),
            ("done", 3),
            ("skipped", 3),
        ]);
        let mut updates = 0usize;
        let mut stmt = remote.prepare("SELECT id,status,completed_at,validated_at,validated_by FROM tasks WHERE plan_id=?1").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([plan_id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (task_id, r_status, completed_at, validated_at, validated_by) =
                row.map_err(|e| e.to_string())?;
            let local_status: Option<String> = local
                .query_row("SELECT status FROM tasks WHERE id=?1", [task_id], |rr| {
                    rr.get(0)
                })
                .ok();
            let Some(l_status) = local_status else {
                continue;
            };
            if rank.get(r_status.as_str()).unwrap_or(&0)
                <= rank.get(l_status.as_str()).unwrap_or(&0)
            {
                continue;
            }
            local.execute(
                "UPDATE tasks SET status=?1, completed_at=COALESCE(?2,completed_at), validated_at=COALESCE(?3,validated_at), validated_by=COALESCE(?4,validated_by) WHERE id=?5",
                params![r_status, completed_at, validated_at, if r_status == "done" { validated_by.or(Some("forced-admin".to_string())) } else { None }, task_id],
            ).map_err(|e| e.to_string())?;
            updates += 1;
        }
        local.execute("UPDATE waves SET tasks_done=(SELECT COUNT(*) FROM tasks WHERE wave_id_fk=waves.id AND status='done') WHERE plan_id=?1", [plan_id]).map_err(|e| e.to_string())?;
        local.execute("UPDATE plans SET tasks_done=(SELECT COUNT(*) FROM tasks WHERE plan_id=?1 AND status='done') WHERE id=?1", [plan_id]).map_err(|e| e.to_string())?;
        updates
    };
    Ok(updates)
}
