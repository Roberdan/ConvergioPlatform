// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Telegram inbound text — long polling via getUpdates (no webhook, M1 Pro behind NAT).
// Security: only processes messages from CONVERGIO_TELEGRAM_CHAT_ID.

use crate::kernel::engine::KernelEngine;
use crate::kernel::telegram_conv;
use crate::kernel::voice_router::{classify_intent, route_intent, VoiceIntent};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Instant};
use tracing::{debug, info, warn};

// ----- Types -----------------------------------------------------------------

/// Minimal Telegram update envelope — only fields we need.
pub struct TelegramUpdate {
    pub update_id: i64,
    pub message: Option<TelegramMessage>,
}

pub struct TelegramMessage {
    pub chat_id: i64,
    pub text: Option<String>,
}

/// Wire representation from getUpdates JSON.
#[derive(Deserialize)]
struct ApiUpdate {
    update_id: i64,
    message: Option<ApiMessage>,
}

#[derive(Deserialize)]
struct ApiMessage {
    chat: ApiChat,
    text: Option<String>,
}

#[derive(Deserialize)]
struct ApiChat {
    id: i64,
}

#[derive(Deserialize)]
struct GetUpdatesResponse {
    ok: bool,
    result: Vec<ApiUpdate>,
}

// ----- Public helpers (testable) --------------------------------------------

/// Extract text from an update only when it comes from the authorised chat_id.
/// Returns None for any other chat (security filter) or non-text updates.
pub fn extract_text_message(update: &TelegramUpdate, chat_id: i64) -> Option<&str> {
    let msg = update.message.as_ref()?;
    if msg.chat_id != chat_id {
        return None;
    }
    msg.text.as_deref()
}

/// Format a Markdown status reply matching the spec example.
pub fn build_status_reply(
    active_plans: u32,
    queued_tasks: u32,
    mesh: &str,
    cost: &str,
) -> String {
    format!(
        "*Jarvis Status*\nPiani attivi: {active_plans}\nTask in coda: {queued_tasks}\nMesh: {mesh}\nCosto oggi: {cost}"
    )
}

// ----- Background polling task ----------------------------------------------

/// Spawn the long-polling loop as a background tokio task.
/// Polls getUpdates every 5 s; rate-limits replies to max 1/s.
/// Returns a JoinHandle so the caller can abort on shutdown.
pub fn spawn_telegram_poll(
    token: String,
    chat_id: i64,
    daemon_url: String,
    engine: Arc<Mutex<KernelEngine>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        info!("jarvis.telegram: starting for chat_id={chat_id}");
        run_poll_loop(&token, chat_id, &daemon_url, &engine).await;
    })
}

async fn run_poll_loop(
    token: &str,
    chat_id: i64,
    daemon_url: &str,
    engine: &Arc<Mutex<KernelEngine>>,
) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(45)) // must exceed Telegram long poll timeout (30s)
        .build()
        .unwrap_or_default();

    let base = format!("https://api.telegram.org/bot{token}");
    let mut offset: i64 = 0;
    // Rate limiter: track last reply time.
    let mut last_reply = Instant::now() - Duration::from_secs(2);

    loop {
        match fetch_updates(&client, &base, offset).await {
            Ok(updates) => {
                for api_upd in &updates {
                    let upd = TelegramUpdate {
                        update_id: api_upd.update_id,
                        message: api_upd.message.as_ref().map(|m| TelegramMessage {
                            chat_id: m.chat.id,
                            text: m.text.clone(),
                        }),
                    };
                    if let Some(text) = extract_text_message(&upd, chat_id) {
                        // Rate limit: ensure >= 1 s between replies.
                        let elapsed = last_reply.elapsed();
                        if elapsed < Duration::from_secs(1) {
                            sleep(Duration::from_secs(1) - elapsed).await;
                        }
                        let reply = process_text(text, chat_id, daemon_url, engine).await;
                        if let Err(e) = send_message(&client, &base, chat_id, &reply).await {
                            warn!("jarvis.telegram: send_message failed: {e}");
                        } else {
                            // Record the exchange for conversation memory
                            telegram_conv::record_exchange(
                                chat_id,
                                text.to_string(),
                                reply.clone(),
                            );
                            debug!("jarvis.telegram: replied to update_id={}", api_upd.update_id);
                        }
                        last_reply = Instant::now();
                    }
                    // Advance offset to avoid reprocessing — NON-NEGOTIABLE.
                    offset = api_upd.update_id + 1;
                }
            }
            Err(e) => {
                warn!("jarvis.telegram: getUpdates error: {e}");
                sleep(Duration::from_secs(5)).await; // backoff on error only
            }
        }
        // No sleep on success — long poll (timeout=30) already waits for messages.
    }
}

async fn process_text(
    text: &str,
    chat_id: i64,
    daemon_url: &str,
    engine: &Arc<Mutex<KernelEngine>>,
) -> String {
    let intent = {
        let eng = engine.lock().unwrap_or_else(|p| p.into_inner());
        classify_intent(text, &eng)
    };
    debug!(?intent, text, "jarvis.telegram: classified intent");

    if matches!(intent, VoiceIntent::EscalateToAli { .. }) {
        let history = telegram_conv::format_history_chatml(chat_id);
        let question = text.to_string();

        let level = {
            let eng = engine.lock().unwrap_or_else(|p| p.into_inner());
            eng.inference_level_for(&question)
        };

        if level == crate::kernel::engine::InferenceLevel::Local {
            let engine_clone = Arc::clone(engine);
            let q = question.clone();
            let h = history.clone();
            return match tokio::task::spawn_blocking(move || {
                let eng = engine_clone.lock().unwrap_or_else(|p| p.into_inner());
                eng.ask_with_history(&q, &h)
            })
            .await
            {
                Ok(r) => r,
                Err(e) => format!("Errore locale: {e}"),
            };
        }

        info!("jarvis.telegram: escalating to cloud");
        return crate::kernel::cloud_escalation::cloud_ask_with_tools(&question, &history).await;
    }

    // route_intent uses reqwest::blocking which deadlocks inside tokio runtime.
    // Wrap in spawn_blocking to run on a dedicated thread pool.
    let daemon = daemon_url.to_string();
    let intent_clone = intent.clone();
    match tokio::task::spawn_blocking(move || route_intent(intent_clone, &daemon)).await {
        Ok(response) => response,
        Err(e) => {
            warn!("jarvis.telegram: spawn_blocking failed: {e}");
            "Errore interno del kernel.".to_string()
        }
    }
}

async fn fetch_updates(
    client: &reqwest::Client,
    base: &str,
    offset: i64,
) -> Result<Vec<ApiUpdate>, String> {
    // Long poll: Telegram holds the connection open up to 30s and returns immediately
    // when a message arrives. Zero wasted requests vs polling every 5s.
    let url = format!("{base}/getUpdates?offset={offset}&timeout=30&allowed_updates=[\"message\"]");
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<GetUpdatesResponse>()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok {
        return Err("Telegram API returned ok=false".to_string());
    }
    Ok(resp.result)
}

async fn send_message(
    client: &reqwest::Client,
    base: &str,
    chat_id: i64,
    text: &str,
) -> Result<(), String> {
    let url = format!("{base}/sendMessage");
    let body = serde_json::json!({
        "chat_id": chat_id,
        "text": text,
        "parse_mode": "Markdown"
    });
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("sendMessage HTTP {}", resp.status()));
    }
    Ok(())
}
