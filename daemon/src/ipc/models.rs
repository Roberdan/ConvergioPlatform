use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

// --- T8053: Ollama probe ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub quantization_level: String,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    #[serde(default)]
    models: Vec<OllamaTagsModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsModel {
    name: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    details: Option<OllamaDetails>,
}

#[derive(Debug, Deserialize)]
struct OllamaDetails {
    #[serde(default)]
    quantization_level: String,
}

pub async fn probe_ollama() -> Result<Vec<OllamaModel>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let resp = client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .map_err(|e| format!("ollama probe: {e}"))?;
    let tags: OllamaTagsResponse = resp.json().await.map_err(|e| format!("parse: {e}"))?;
    Ok(tags
        .models
        .into_iter()
        .map(|m| OllamaModel {
            name: m.name,
            size: m.size,
            quantization_level: m.details.map(|d| d.quantization_level).unwrap_or_default(),
        })
        .collect())
}

pub async fn probe_lmstudio() -> Result<Vec<OllamaModel>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("http client: {e}"))?;
    let resp = client
        .get("http://localhost:1234/v1/models")
        .send()
        .await
        .map_err(|e| format!("lmstudio probe: {e}"))?;
    let body: serde_json::Value = resp.json().await.map_err(|e| format!("parse: {e}"))?;
    let models = body["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|m| OllamaModel {
                    name: m["id"].as_str().unwrap_or("unknown").to_string(),
                    size: 0,
                    quantization_level: String::new(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(models)
}

pub fn store_models(
    conn: &Connection,
    host: &str,
    provider: &str,
    models: &[OllamaModel],
) -> rusqlite::Result<()> {
    for m in models {
        let size_gb = m.size as f64 / 1_073_741_824.0;
        conn.execute(
            "INSERT OR REPLACE INTO ipc_model_registry (host, provider, model, size_gb, quantization, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
            params![host, provider, m.name, size_gb, m.quantization_level],
        )?;
    }
    Ok(())
}

pub fn get_all_models(conn: &Connection) -> rusqlite::Result<Vec<ModelEntry>> {
    let mut stmt = conn.prepare(
        "SELECT host, provider, model, size_gb, quantization, last_seen
         FROM ipc_model_registry ORDER BY host, provider, model",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ModelEntry {
            host: row.get(0)?,
            provider: row.get(1)?,
            model: row.get(2)?,
            size_gb: row.get(3)?,
            quantization: row.get(4)?,
            last_seen: row.get(5)?,
        })
    })?;
    rows.collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelEntry {
    pub host: String,
    pub provider: String,
    pub model: String,
    pub size_gb: f64,
    pub quantization: String,
    pub last_seen: String,
}

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

#[derive(Debug, Clone, Serialize)]
pub struct NodeCapabilities {
    pub host: String,
    pub provider: String,
    pub models: Vec<String>,
    pub updated_at: String,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("
            CREATE TABLE ipc_model_registry (id INTEGER PRIMARY KEY, host TEXT, provider TEXT, model TEXT, size_gb REAL, quantization TEXT, last_seen TEXT, UNIQUE(host,provider,model));
            CREATE TABLE ipc_node_capabilities (host TEXT PRIMARY KEY, provider TEXT, models TEXT, updated_at TEXT);
            CREATE TABLE ipc_subscriptions (name TEXT PRIMARY KEY, provider TEXT, plan TEXT, budget_usd REAL, reset_day INTEGER, models TEXT);
        ").unwrap();
        conn
    }

    #[test]
    fn test_store_and_query_models() {
        let conn = setup_db();
        let models = vec![
            OllamaModel {
                name: "llama3".into(),
                size: 7_000_000_000,
                quantization_level: "Q4".into(),
            },
            OllamaModel {
                name: "codellama".into(),
                size: 13_000_000_000,
                quantization_level: "Q8".into(),
            },
        ];
        store_models(&conn, "mac-worker-2", "ollama", &models).unwrap();
        let all = get_all_models(&conn).unwrap();
        assert_eq!(all.len(), 2);
        assert!(all[0].size_gb > 0.0);
    }

    #[test]
    fn test_advertise_capabilities() {
        let conn = setup_db();
        let m = vec![OllamaModel {
            name: "m1".into(),
            size: 0,
            quantization_level: "".into(),
        }];
        store_models(&conn, "host1", "ollama", &m).unwrap();
        advertise_capabilities(&conn, "host1").unwrap();
        let caps = get_all_capabilities(&conn).unwrap();
        assert_eq!(caps.len(), 1);
        assert!(caps[0].models.contains(&"m1".to_string()));
    }

    #[test]
    fn test_subscription_crud() {
        let conn = setup_db();
        let sub = Subscription {
            name: "openai-pro".into(),
            provider: "openai".into(),
            plan: "pro".into(),
            budget_usd: 100.0,
            reset_day: 1,
            models: vec!["gpt-4o".into()],
        };
        add_subscription(&conn, &sub).unwrap();
        let subs = list_subscriptions(&conn).unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].name, "openai-pro");
        remove_subscription(&conn, "openai-pro").unwrap();
        assert_eq!(list_subscriptions(&conn).unwrap().len(), 0);
    }

    #[test]
    fn test_model_entry_fields() {
        let conn = setup_db();
        let m = vec![OllamaModel {
            name: "test".into(),
            size: 5_368_709_120,
            quantization_level: "Q5".into(),
        }];
        store_models(&conn, "h", "lmstudio", &m).unwrap();
        let all = get_all_models(&conn).unwrap();
        assert_eq!(all[0].provider, "lmstudio");
        assert!((all[0].size_gb - 5.0).abs() < 0.1);
    }
}
