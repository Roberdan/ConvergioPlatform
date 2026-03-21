// nightly_data: optimize signals and clear handlers
use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;

pub async fn api_optimize_signals() -> Json<Value> {
    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let path = format!("{home}/.claude/data/session-learnings.jsonl");
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    if content.trim().is_empty() {
        return Json(json!({"count": 0, "signals": [], "by_type": []}));
    }
    let entries: Vec<Value> = content
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    let count = entries.len();
    let mut type_counts: HashMap<String, (i64, Vec<Value>)> = HashMap::new();
    for entry in &entries {
        if let Some(signals) = entry.get("signals").and_then(Value::as_array) {
            for sig in signals {
                let sig_type = sig
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                let e = type_counts.entry(sig_type).or_insert((0, Vec::new()));
                e.0 += 1;
                if e.1.len() < 3 {
                    e.1.push(sig.get("data").cloned().unwrap_or(Value::Null));
                }
            }
        }
    }
    let by_type: Vec<Value> = type_counts
        .into_iter()
        .map(|(t, (c, samples))| json!({"type": t, "count": c, "samples": samples}))
        .collect();
    let projects: Vec<String> = entries
        .iter()
        .filter_map(|e| e.get("project").and_then(Value::as_str).map(String::from))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    Json(json!({
        "count": count,
        "signals": entries,
        "by_type": by_type,
        "projects": projects,
    }))
}

pub async fn api_optimize_clear() -> Json<Value> {
    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let src = format!("{home}/.claude/data/session-learnings.jsonl");
    let archive = format!("{home}/.claude/data/session-learnings-archive.jsonl");
    let content = std::fs::read_to_string(&src).unwrap_or_default();
    if !content.is_empty() {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&archive)
        {
            let _ = f.write_all(content.as_bytes());
        }
        let _ = std::fs::write(&src, "");
    }
    Json(json!({"ok": true, "archived": !content.is_empty()}))
}
