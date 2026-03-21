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

// --- T8055: Capability advertisement types ---

#[derive(Debug, Clone, Serialize)]
pub struct NodeCapabilities {
    pub host: String,
    pub provider: String,
    pub models: Vec<String>,
    pub updated_at: String,
}
