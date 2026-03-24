// Handlers — one function per orchestration event type.

use crate::db::plan_hierarchy;
use crate::ipc::IpcEngine;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::actions;

type AliResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// plan_started / plan_ready — check dependencies, delegate if met.
pub async fn on_plan_ready(
    engine: &Arc<IpcEngine>,
    db_path: &PathBuf,
    plan_id: i64,
) -> AliResult {
    let conn = rusqlite::Connection::open(db_path)?;
    let deps_met = plan_hierarchy::dependencies_met(&conn, plan_id)?;

    if !deps_met {
        tracing::info!("ali: plan {plan_id} blocked — dependencies not met");
        actions::emit(engine, "plan_blocked", &serde_json::json!({"plan_id": plan_id}))?;
        return Ok(());
    }

    tracing::info!("ali: plan {plan_id} ready — delegating");
    actions::delegate_plan(engine, db_path, plan_id).await
}

/// task_done — check if wave is complete.
pub async fn on_task_done(
    engine: &Arc<IpcEngine>,
    db_path: &PathBuf,
    task_id: &str,
    plan_id: i64,
) -> AliResult {
    let conn = rusqlite::Connection::open(db_path)?;

    // Count pending tasks in the same wave
    let wave_id: Option<i64> = conn
        .query_row(
            "SELECT wave_id FROM tasks WHERE id = ?1 AND plan_id = ?2",
            rusqlite::params![task_id, plan_id],
            |row| row.get(0),
        )
        .ok();

    let Some(wave_id) = wave_id else {
        tracing::warn!("ali: task {task_id} not found in plan {plan_id}");
        return Ok(());
    };

    let pending: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE plan_id = ?1 AND wave_id = ?2 AND status NOT IN ('done', 'cancelled', 'skipped')",
        rusqlite::params![plan_id, wave_id],
        |row| row.get(0),
    )?;

    tracing::info!("ali: task {task_id} done, wave {wave_id} has {pending} remaining");

    if pending == 0 {
        actions::emit(
            engine,
            "wave_done",
            &serde_json::json!({"wave_id": wave_id, "plan_id": plan_id}),
        )?;
    }

    Ok(())
}

/// wave_done — emit validation request (Thor will pick this up).
pub fn on_wave_done(engine: &Arc<IpcEngine>, wave_id: i64, plan_id: i64) -> AliResult {
    tracing::info!("ali: wave {wave_id} complete for plan {plan_id}, requesting validation");
    actions::emit(
        engine,
        "wave_needs_validation",
        &serde_json::json!({"wave_id": wave_id, "plan_id": plan_id}),
    )?;
    Ok(())
}

/// wave_validated — advance to next wave or complete plan.
pub fn on_wave_validated(
    engine: &Arc<IpcEngine>,
    db_path: &Path,
    wave_id: i64,
    plan_id: i64,
) -> AliResult {
    let conn = rusqlite::Connection::open(db_path)?;

    // Check if there are more waves after this one
    let next_wave: Option<i64> = conn
        .query_row(
            "SELECT id FROM waves WHERE plan_id = ?1 AND id > ?2 AND status = 'pending' ORDER BY id LIMIT 1",
            rusqlite::params![plan_id, wave_id],
            |row| row.get(0),
        )
        .ok();

    if let Some(next) = next_wave {
        tracing::info!("ali: wave {wave_id} validated, next wave {next} for plan {plan_id}");
        actions::emit(
            engine,
            "wave_ready",
            &serde_json::json!({"wave_id": next, "plan_id": plan_id}),
        )?;
    } else {
        tracing::info!("ali: all waves validated for plan {plan_id}, plan done");
        actions::emit(
            engine,
            "plan_done",
            &serde_json::json!({"plan_id": plan_id}),
        )?;
    }

    Ok(())
}

/// plan_done — rollup master, unblock sibling plans.
pub fn on_plan_done(engine: &Arc<IpcEngine>, db_path: &Path, plan_id: i64) -> AliResult {
    let conn = rusqlite::Connection::open(db_path)?;

    // Find parent (master) plan
    let parent_id: Option<i64> = conn
        .query_row(
            "SELECT parent_plan_id FROM plans WHERE id = ?1",
            rusqlite::params![plan_id],
            |row| row.get(0),
        )
        .ok()
        .flatten();

    if let Some(master_id) = parent_id {
        let (done, total, status) = plan_hierarchy::master_rollup(&conn, master_id)?;
        tracing::info!("ali: master {master_id} rollup: {done}/{total} status={status}");

        actions::check_unblocked_plans(engine, &conn, master_id)?;
    }

    Ok(())
}

/// wave_ready — start executing tasks in this wave by delegating the plan.
pub async fn on_wave_ready(
    engine: &Arc<IpcEngine>,
    db_path: &PathBuf,
    wave_id: i64,
    plan_id: i64,
) -> AliResult {
    let conn = rusqlite::Connection::open(db_path)?;

    // Update wave status to in_progress
    conn.execute(
        "UPDATE waves SET status='in_progress', started_at=datetime('now') WHERE id=?1",
        rusqlite::params![wave_id],
    )?;

    // Count tasks in this wave
    let task_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE plan_id=?1 AND wave_id=?2 AND status='pending'",
        rusqlite::params![plan_id, wave_id],
        |r| r.get(0),
    )?;

    tracing::info!("ali: wave {wave_id} starting with {task_count} pending tasks for plan {plan_id}");

    // Delegate the plan to a peer — the executor will pick up pending tasks
    actions::delegate_plan(engine, db_path, plan_id).await
}

/// delegation_failed — retry on different peer, or escalate to human.
pub async fn on_delegation_failed(
    engine: &Arc<IpcEngine>,
    db_path: &PathBuf,
    plan_id: i64,
    failed_peer: &str,
    reason: &str,
) -> AliResult {
    tracing::warn!("ali: delegation of plan {plan_id} to {failed_peer} failed: {reason}");

    // Try finding a different peer
    let alt_peer = actions::find_available_peer(db_path, Some(failed_peer)).await;

    if let Some(peer) = alt_peer {
        tracing::info!("ali: retrying plan {plan_id} on peer {peer}");
        actions::delegate_to_peer(engine, plan_id, &peer).await
    } else {
        tracing::warn!("ali: no peers available for plan {plan_id}, requesting human intervention");
        actions::emit(
            engine,
            "need_human",
            &serde_json::json!({
                "plan_id": plan_id,
                "reason": format!("delegation failed on {failed_peer}: {reason}, no alternative peers"),
            }),
        )?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "handlers_tests.rs"]
mod tests;
