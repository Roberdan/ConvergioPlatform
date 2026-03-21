use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::types::{probe_lmstudio, probe_ollama, store_models, NodeCapabilities};

// --- T8054: Periodic probe loop ---

pub async fn start_model_probe(conn_path: std::path::PathBuf, interval_secs: u64) {
    let interval = std::time::Duration::from_secs(interval_secs);
    let host = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    loop {
        if let Ok(conn) = rusqlite::Connection::open(&conn_path) {
            if let Ok(models) = probe_ollama().await {
                let _ = store_models(&conn, &host, "ollama", &models);
            }
            if let Ok(models) = probe_lmstudio().await {
                let _ = store_models(&conn, &host, "lmstudio", &models);
            }
            let _ = advertise_capabilities(&conn, &host);
        }
        tokio::time::sleep(interval).await;
    }
}

// --- T8055: Capability advertisement ---

pub fn advertise_capabilities(conn: &Connection, host: &str) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(
        "SELECT provider, GROUP_CONCAT(model) FROM ipc_model_registry
         WHERE host=?1 GROUP BY provider",
    )?;
    let providers: Vec<(String, String)> = stmt
        .query_map(params![host], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    for (provider, models_csv) in &providers {
        let models_json: Vec<&str> = models_csv.split(',').collect();
        let json = serde_json::to_string(&models_json).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT OR REPLACE INTO ipc_node_capabilities (host, provider, models, updated_at)
             VALUES (?1, ?2, ?3, datetime('now'))",
            params![host, provider, json],
        )?;
    }
    Ok(())
}

pub fn get_all_capabilities(conn: &Connection) -> rusqlite::Result<Vec<NodeCapabilities>> {
    let mut stmt = conn.prepare(
        "SELECT host, provider, models, updated_at FROM ipc_node_capabilities ORDER BY host",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(NodeCapabilities {
            host: row.get(0)?,
            provider: row.get(1)?,
            models: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(2)?)
                .unwrap_or_default(),
            updated_at: row.get(3)?,
        })
    })?;
    rows.collect()
}

// --- T8056: Subscription registry ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub name: String,
    pub provider: String,
    pub plan: String,
    pub budget_usd: f64,
    pub reset_day: i32,
    pub models: Vec<String>,
}

pub fn add_subscription(conn: &Connection, sub: &Subscription) -> rusqlite::Result<()> {
    let models_json = serde_json::to_string(&sub.models).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "INSERT OR REPLACE INTO ipc_subscriptions (name, provider, plan, budget_usd, reset_day, models)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![sub.name, sub.provider, sub.plan, sub.budget_usd, sub.reset_day, models_json],
    )?;
    Ok(())
}

pub fn list_subscriptions(conn: &Connection) -> rusqlite::Result<Vec<Subscription>> {
    let mut stmt = conn.prepare(
        "SELECT name, provider, plan, budget_usd, reset_day, models FROM ipc_subscriptions ORDER BY name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Subscription {
            name: row.get(0)?,
            provider: row.get(1)?,
            plan: row.get(2)?,
            budget_usd: row.get(3)?,
            reset_day: row.get(4)?,
            models: serde_json::from_str::<Vec<String>>(&row.get::<_, String>(5)?)
                .unwrap_or_default(),
        })
    })?;
    rows.collect()
}

pub fn remove_subscription(conn: &Connection, name: &str) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM ipc_subscriptions WHERE name=?1", params![name])
}

// --- T8058: Health checks ---

#[derive(Debug, Clone, Serialize)]
pub struct ProviderHealth {
    pub provider: String,
    pub reachable: bool,
    pub latency_ms: u64,
}

pub async fn health_check_providers() -> Vec<ProviderHealth> {
    let mut results = Vec::new();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // Ollama
    let start = std::time::Instant::now();
    let ollama_ok = client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .is_ok();
    results.push(ProviderHealth {
        provider: "ollama".to_string(),
        reachable: ollama_ok,
        latency_ms: start.elapsed().as_millis() as u64,
    });

    // LMStudio
    let start = std::time::Instant::now();
    let lms_ok = client
        .get("http://localhost:1234/v1/models")
        .send()
        .await
        .is_ok();
    results.push(ProviderHealth {
        provider: "lmstudio".to_string(),
        reachable: lms_ok,
        latency_ms: start.elapsed().as_millis() as u64,
    });

    results
}
