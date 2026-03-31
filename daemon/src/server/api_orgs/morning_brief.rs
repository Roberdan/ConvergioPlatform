use crate::server::api_ipc::ensure_ipc_schema;
use crate::server::state::{query_rows, ApiError, ServerState};
use axum::extract::State;
use axum::Json;
use chrono::{Duration, Local, TimeZone};
use serde_json::{json, Value};
use std::sync::OnceLock;
use tokio::time::sleep;

static MORNING_BRIEF_STARTED: OnceLock<()> = OnceLock::new();

pub fn spawn_morning_brief_cron(state: ServerState) {
    if MORNING_BRIEF_STARTED.set(()).is_err() {
        return;
    }
    tokio::spawn(async move {
        loop {
            sleep(seconds_until_0645_local()).await;
            let _ = run_morning_brief_once(&state).await;
        }
    });
}

pub async fn get_morning_brief(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(run_morning_brief_once(&state).await?))
}

pub async fn run_morning_brief_once(state: &ServerState) -> Result<Value, ApiError> {
    ensure_ipc_schema(state)?;
    let conn = state.get_conn()?;
    let orgs = query_rows(
        &conn,
        "SELECT id FROM ipc_orgs WHERE status='active' ORDER BY id",
        [],
    )?;
    for org in &orgs {
        if let Some(org_id) = org.get("id").and_then(|v| v.as_str()) {
            let digest = super::digest::generate_digest_for_org(&conn, org_id)?;
            let _ = conn.execute(
                "INSERT INTO ipc_org_digests(id, org_id, type, content) VALUES (?1, ?2, 'morning', ?3)",
                rusqlite::params![format!("digest-{}", uuid::Uuid::new_v4().simple()), org_id, digest.to_string()],
            );
        }
    }
    let escalations = query_rows(
        &conn,
        "SELECT org_id, id, decision, created_at
         FROM ipc_decisions
         WHERE created_at <= datetime('now', '-4 hours')
         ORDER BY created_at ASC",
        [],
    )?;
    let brief = json!({
        "org_count": orgs.len(),
        "escalations": escalations,
        "orgs": orgs,
    });
    send_morning_telegram(&format_morning_message(&brief)).await;
    Ok(brief)
}

fn seconds_until_0645_local() -> std::time::Duration {
    let now = Local::now();
    let today = now.date_naive();
    let run_today = today
        .and_hms_opt(6, 45, 0)
        .and_then(|ts| Local.from_local_datetime(&ts).single());
    let next = match run_today {
        Some(run) if run > now => run,
        _ => {
            let tomorrow = today + Duration::days(1);
            let ts = tomorrow.and_hms_opt(6, 45, 0).expect("valid time");
            Local.from_local_datetime(&ts).single().unwrap_or(now + Duration::hours(24))
        }
    };
    let secs = (next - now).num_seconds().max(60) as u64;
    std::time::Duration::from_secs(secs)
}

fn format_morning_message(brief: &Value) -> String {
    let org_count = brief["org_count"].as_u64().unwrap_or(0);
    let escalations = brief["escalations"].as_array().map(|a| a.len()).unwrap_or(0);
    format!("🌅 Buongiorno Roberto — {org_count} org attive\n⚠️ Escalations aperte: {escalations}")
}

async fn send_morning_telegram(text: &str) {
    let token = crate::telegram_config::telegram_token();
    let chat_id = crate::telegram_config::telegram_chat_id().ok().flatten();
    if let (Some(token), Some(chat_id)) = (token, chat_id) {
        let _ = crate::kernel::telegram::send_text(&token, chat_id, text, None).await;
    }
}
