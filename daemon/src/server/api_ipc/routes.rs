// routes: Plan 635 intelligence endpoints — budget, models, skills, auth, metrics, logs
use super::super::state::{ApiError, ServerState};
use super::IPC_LOG_BUFFER;
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

pub async fn api_ipc_budget(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;
    let subs = crate::ipc::models::list_subscriptions(conn)
        .map_err(|e| ApiError::internal(format!("subs: {e}")))?;
    let mut budgets = Vec::new();
    for s in &subs {
        let status = crate::ipc::budget::get_budget_status(conn, &s.name)
            .map_err(|e| ApiError::internal(format!("budget: {e}")))?;
        let alert = crate::ipc::budget::check_budget_thresholds(conn, &s.name)
            .map_err(|e| ApiError::internal(format!("alert: {e}")))?;
        budgets.push(json!({
            "subscription": s.name, "provider": s.provider,
            "plan": s.plan, "budget_usd": s.budget_usd,
            "status": status, "alert": alert,
        }));
    }
    Ok(Json(json!({ "budgets": budgets })))
}

pub async fn api_ipc_models(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;
    let models = crate::ipc::models::get_all_models(conn)
        .map_err(|e| ApiError::internal(format!("models: {e}")))?;
    let capabilities = crate::ipc::models::get_all_capabilities(conn)
        .map_err(|e| ApiError::internal(format!("caps: {e}")))?;
    Ok(Json(json!({ "models": models, "capabilities": capabilities })))
}

pub async fn api_ipc_skills(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;
    let pool = crate::ipc::skills::get_skill_pool(conn)
        .map_err(|e| ApiError::internal(format!("skills: {e}")))?;
    let flat: Vec<_> = pool.values().flatten().collect();
    Ok(Json(json!({ "skills": flat })))
}

pub async fn api_ipc_auth_status(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;
    let health = crate::ipc::auth_sync::check_token_sync_health(conn)
        .map_err(|e| ApiError::internal(format!("health: {e}")))?;
    let tokens = crate::ipc::auth_sync::list_tokens(conn)
        .map_err(|e| ApiError::internal(format!("tokens: {e}")))?;
    Ok(Json(json!({ "health": health, "tokens": tokens })))
}

pub async fn api_ipc_route_history(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;
    let mut stmt = conn
        .prepare(
            "SELECT subscription, date, tokens_in, tokens_out, estimated_cost_usd, model, task_ref
             FROM ipc_budget_log ORDER BY id DESC LIMIT 20",
        )
        .map_err(|e| ApiError::internal(format!("prepare: {e}")))?;
    let entries: Vec<Value> = stmt
        .query_map([], |row| {
            Ok(json!({
                "subscription": row.get::<_, String>(0)?,
                "date": row.get::<_, String>(1)?,
                "tokens_in": row.get::<_, i64>(2)?,
                "tokens_out": row.get::<_, i64>(3)?,
                "cost": row.get::<_, f64>(4)?,
                "model": row.get::<_, String>(5)?,
                "task": row.get::<_, String>(6)?,
            }))
        })
        .map_err(|e| ApiError::internal(format!("query: {e}")))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Json(json!({ "history": entries })))
}

pub async fn api_ipc_metrics(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;
    let model_count: i64 = conn
        .query_row("SELECT count(*) FROM ipc_model_registry", [], |r| r.get(0))
        .unwrap_or(0);
    let agent_count: i64 = conn
        .query_row(
            "SELECT count(DISTINCT agent) FROM ipc_agent_skills WHERE agent != ''",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let ipc_message_rate: i64 = conn
        .query_row(
            "SELECT count(*) FROM ipc_budget_log WHERE date >= date('now', '-1 day')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let budget_usage: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(estimated_cost_usd), 0) FROM ipc_budget_log",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0.0);
    let skill_requests_active: i64 = conn.query_row(
        "SELECT count(*) FROM session_state WHERE key LIKE 'skill_req:%' AND value LIKE '%pending%'",
        [], |r| r.get(0),
    ).unwrap_or(0);
    Ok(Json(json!({
        "ipc_message_rate_1d": ipc_message_rate,
        "agent_count": agent_count,
        "model_count": model_count,
        "avg_route_latency_ms": 0,
        "budget_usage": budget_usage,
        "skill_requests_active": skill_requests_active,
    })))
}

#[derive(Deserialize)]
pub struct LogsQuery {
    limit: Option<usize>,
}

pub async fn api_ipc_logs(Query(q): Query<LogsQuery>) -> Result<Json<Value>, ApiError> {
    let limit = q.limit.unwrap_or(100).min(1000);
    let guard = IPC_LOG_BUFFER
        .lock()
        .map_err(|e| ApiError::internal(format!("lock: {e}")))?;
    let entries: Vec<Value> = guard
        .as_ref()
        .map(|buf| {
            buf.iter()
                .rev()
                .take(limit)
                .map(|e| {
                    json!({
                        "timestamp": e.timestamp, "level": e.level,
                        "module": e.module, "message": e.message,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Json(json!({ "logs": entries, "count": entries.len() })))
}
