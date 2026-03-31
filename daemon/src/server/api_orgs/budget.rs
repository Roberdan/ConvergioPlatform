use crate::server::state::{ApiError, ServerState};
use rusqlite::{params, Connection};
use serde_json::json;
use uuid::Uuid;

const MEMBER_ACTION_TOKENS: i64 = 1;

fn ensure_budget_schema(conn: &Connection) {
    let _ = conn.execute(
        "ALTER TABLE ipc_orgs ADD COLUMN daily_budget_tokens INTEGER NOT NULL DEFAULT 1000",
        [],
    );
}

pub fn guard_member_action_budget(
    state: &ServerState,
    conn: &Connection,
    org_id: &str,
    action: &str,
) -> Result<(), ApiError> {
    ensure_budget_schema(conn);
    let (status, daily_budget, ceo_agent): (String, i64, String) = conn
        .query_row(
            "SELECT status, daily_budget_tokens, ceo_agent FROM ipc_orgs WHERE id = ?1",
            params![org_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| ApiError::not_found("org not found"))?;
    if status == "suspended" {
        return Err(ApiError::forbidden("organization suspended by budget guard"));
    }

    let ten_min_tokens: i64 = conn
        .query_row(
            "SELECT CAST(COALESCE(SUM(value), 0) AS INTEGER) FROM ipc_org_telemetry
             WHERE org_id = ?1
               AND metric = 'tokens_consumed'
               AND julianday(created_at) >= julianday('now', '-10 minutes')",
            params![org_id],
            |row| row.get(0),
        )
        .map_err(|e| ApiError::internal(format!("budget query failed: {e}")))?;
    if ten_min_tokens + MEMBER_ACTION_TOKENS > daily_budget.saturating_mul(3) {
        conn.execute(
            "UPDATE ipc_orgs
             SET status = 'suspended',
                 updated_at = (strftime('%Y-%m-%dT%H:%M:%f','now'))
             WHERE id = ?1",
            params![org_id],
        )
        .map_err(|e| ApiError::internal(format!("org suspension failed: {e}")))?;
        if let Some(ref ipc) = state.ipc_engine {
            let alert = json!({
                "type": "budget_circuit_breaker",
                "org_id": org_id,
                "action": action,
                "daily_budget_tokens": daily_budget,
                "tokens_10m": ten_min_tokens + MEMBER_ACTION_TOKENS
            })
            .to_string();
            if let Err(e) = ipc.broadcast(&ceo_agent, &alert, "event", Some(&format!("org:{org_id}"))) {
                tracing::warn!("org budget breaker broadcast failed: {e}");
            }
        }
        return Err(ApiError::forbidden("organization suspended by budget circuit breaker"));
    }

    let daily_tokens: i64 = conn
        .query_row(
            "SELECT CAST(COALESCE(SUM(value), 0) AS INTEGER) FROM ipc_org_telemetry
             WHERE org_id = ?1
               AND metric = 'tokens_consumed'
               AND date(created_at) = date('now')",
            params![org_id],
            |row| row.get(0),
        )
        .map_err(|e| ApiError::internal(format!("daily budget query failed: {e}")))?;
    if daily_tokens + MEMBER_ACTION_TOKENS > daily_budget {
        return Err(ApiError::rate_limited("organization daily token budget exceeded"));
    }

    conn.execute(
        "INSERT INTO ipc_org_telemetry(id, org_id, metric, value, tags)
         VALUES (?1, ?2, 'tokens_consumed', ?3, ?4)",
        params![
            format!("telemetry-{}", Uuid::new_v4().simple()),
            org_id,
            MEMBER_ACTION_TOKENS,
            json!({ "action": action }).to_string()
        ],
    )
    .map_err(|e| ApiError::internal(format!("record token telemetry failed: {e}")))?;
    Ok(())
}
