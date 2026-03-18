use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tracing::warn;

// --- T8066: Task analysis ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    CodeGen,
    Refactor,
    Architecture,
    Testing,
    Documentation,
    SecurityReview,
    QuickExploration,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskAnalysis {
    pub task_type: TaskType,
    pub complexity: f64,
    pub estimated_tokens: i64,
    pub keywords: Vec<String>,
}

pub fn analyze_task(description: &str) -> TaskAnalysis {
    tracing::info!("analyzing task for routing");
    let lower = description.to_lowercase();
    let mut keywords = Vec::new();
    let task_type = if lower.contains("security")
        || lower.contains("audit")
        || lower.contains("vulnerability")
    {
        keywords.extend(["security", "audit"].iter().map(|s| s.to_string()));
        TaskType::SecurityReview
    } else if lower.contains("architect") || lower.contains("design") || lower.contains("system") {
        keywords.extend(["architecture", "design"].iter().map(|s| s.to_string()));
        TaskType::Architecture
    } else if lower.contains("refactor")
        || lower.contains("rename")
        || lower.contains("restructure")
    {
        keywords.extend(["refactor", "rename"].iter().map(|s| s.to_string()));
        TaskType::Refactor
    } else if lower.contains("test") || lower.contains("spec") || lower.contains("assert") {
        keywords.extend(["test", "spec"].iter().map(|s| s.to_string()));
        TaskType::Testing
    } else if lower.contains("doc") || lower.contains("readme") || lower.contains("comment") {
        keywords.extend(["documentation"].iter().map(|s| s.to_string()));
        TaskType::Documentation
    } else if lower.contains("find")
        || lower.contains("search")
        || lower.contains("explore")
        || lower.contains("list")
    {
        keywords.extend(["explore", "search"].iter().map(|s| s.to_string()));
        TaskType::QuickExploration
    } else {
        keywords.extend(["code", "implement"].iter().map(|s| s.to_string()));
        TaskType::CodeGen
    };
    let words = description.split_whitespace().count();
    let complexity = (words as f64 / 50.0).min(1.0).max(0.1);
    let estimated_tokens = (words as f64 * 1.3 * 4.0) as i64;
    TaskAnalysis {
        task_type,
        complexity,
        estimated_tokens,
        keywords,
    }
}

// --- T8067: Model matching ---

#[derive(Debug, Clone, Serialize)]
pub struct RouteDecision {
    pub model: String,
    pub provider: String,
    pub host: String,
    pub reason: String,
    pub score: f64,
    pub confidence: f64,
    pub estimated_cost: f64,
}

pub fn route_task(conn: &Connection, description: &str) -> rusqlite::Result<Option<RouteDecision>> {
    let analysis = analyze_task(description);
    tracing::info!(task_type = ?analysis.task_type, "routing task");
    let mut stmt = conn.prepare(
        "SELECT r.host, r.provider, r.model, r.size_gb,
                COALESCE(s.budget_usd, 0) as budget,
                COALESCE((SELECT SUM(estimated_cost_usd) FROM ipc_budget_log WHERE subscription=s.name), 0) as spent
         FROM ipc_model_registry r
         LEFT JOIN ipc_subscriptions s ON r.provider = s.provider
         ORDER BY r.host, r.provider",
    )?;
    let candidates: Vec<(String, String, String, f64, f64, f64)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut best: Option<RouteDecision> = None;
    for (host, provider, model, _size, budget, spent) in &candidates {
        let capability_match = match analysis.task_type {
            TaskType::CodeGen | TaskType::Refactor => {
                if model.contains("codex") || model.contains("code") {
                    1.0
                } else {
                    0.7
                }
            }
            TaskType::Architecture | TaskType::SecurityReview => {
                if model.contains("opus") || model.contains("gpt-4") {
                    1.0
                } else {
                    0.5
                }
            }
            _ => 0.6,
        };
        let budget_headroom = if *budget > 0.0 {
            1.0 - (spent / budget).min(1.0)
        } else {
            1.0 // free/local model
        };
        let is_local = provider == "ollama" || provider == "lmstudio";
        let availability = if is_local { 1.0 } else { 0.9 };
        let score = capability_match * budget_headroom * availability;
        let cost = super::budget::estimate_task_cost(description, model);

        if best.as_ref().map_or(true, |b| score > b.score) {
            best = Some(RouteDecision {
                model: model.clone(),
                provider: provider.clone(),
                host: host.clone(),
                reason: format!(
                    "{:?} task → {model} (cap={capability_match:.1} budget={budget_headroom:.1})",
                    analysis.task_type
                ),
                score,
                confidence: score,
                estimated_cost: cost,
            });
        }
    }
    Ok(best)
}

// --- T8069: Auto-fallback ---

#[derive(Debug, Clone, Serialize)]
pub struct FallbackOption {
    pub provider: String,
    pub model: String,
    pub host: String,
    pub is_free: bool,
    pub priority: i32,
    pub degraded: bool,
}

pub fn fallback_chain(
    conn: &Connection,
    primary_sub: &str,
) -> rusqlite::Result<Vec<FallbackOption>> {
    tracing::info!("building fallback chain");
    let mut chain = Vec::new();
    // Priority 1: Local Ollama models (free)
    let mut stmt = conn.prepare(
        "SELECT host, model FROM ipc_model_registry WHERE provider='ollama' ORDER BY size_gb DESC",
    )?;
    let local: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    for (i, (host, model)) in local.iter().enumerate() {
        chain.push(FallbackOption {
            provider: "ollama".into(),
            model: model.clone(),
            host: host.clone(),
            is_free: true,
            priority: i as i32,
            degraded: false,
        });
    }
    // Priority 2: Cloud subs with budget remaining
    let mut stmt = conn.prepare(
        "SELECT s.name, s.provider, s.budget_usd,
                COALESCE((SELECT SUM(estimated_cost_usd) FROM ipc_budget_log WHERE subscription=s.name), 0)
         FROM ipc_subscriptions s WHERE s.name != ?1 ORDER BY s.budget_usd DESC",
    )?;
    let subs: Vec<(String, String, f64, f64)> = stmt
        .query_map(params![primary_sub], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .filter_map(|r| r.ok())
        .collect();
    let base = chain.len() as i32;
    for (i, (name, provider, budget, spent)) in subs.iter().enumerate() {
        if spent < budget {
            chain.push(FallbackOption {
                provider: provider.clone(),
                model: name.clone(),
                host: "cloud".into(),
                is_free: false,
                priority: base + i as i32,
                degraded: false,
            });
        }
    }
    // Priority 3: Degraded mode
    if chain.is_empty() {
        warn!("no models available, falling back to degraded mode");
        chain.push(FallbackOption {
            provider: "none".into(),
            model: "degraded".into(),
            host: "local".into(),
            is_free: true,
            priority: 999,
            degraded: true,
        });
    }
    Ok(chain)
}

// --- T8070: Distributed execution ---

#[derive(Debug, Clone, Serialize)]
pub struct SubtaskAssignment {
    pub subtask: String,
    pub host: String,
    pub model: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionPlan {
    pub subtasks: Vec<SubtaskAssignment>,
    pub parallel: bool,
    pub estimated_total_cost: f64,
}

pub fn plan_parallel_execution(
    conn: &Connection,
    description: &str,
    subtask_count: usize,
) -> rusqlite::Result<ExecutionPlan> {
    let analysis = analyze_task(description);
    let words: Vec<&str> = description
        .split('.')
        .filter(|s| !s.trim().is_empty())
        .collect();
    let parts = if words.len() >= subtask_count {
        words
    } else {
        vec![description; subtask_count]
    };

    let mut assignments = Vec::new();
    let mut total_cost = 0.0;

    for (i, part) in parts.iter().enumerate().take(subtask_count) {
        if let Ok(Some(decision)) = route_task(conn, part) {
            total_cost += decision.estimated_cost;
            assignments.push(SubtaskAssignment {
                subtask: format!("subtask-{}: {}", i, part.trim()),
                host: decision.host,
                model: decision.model,
                provider: decision.provider,
            });
        } else {
            assignments.push(SubtaskAssignment {
                subtask: format!("subtask-{}: {}", i, part.trim()),
                host: "local".into(),
                model: format!("{:?}", analysis.task_type),
                provider: "fallback".into(),
            });
        }
    }

    Ok(ExecutionPlan {
        subtasks: assignments,
        parallel: true,
        estimated_total_cost: total_cost,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("
            CREATE TABLE ipc_model_registry (id INTEGER PRIMARY KEY, host TEXT, provider TEXT, model TEXT, size_gb REAL, quantization TEXT, last_seen TEXT, UNIQUE(host,provider,model));
            CREATE TABLE ipc_subscriptions (name TEXT PRIMARY KEY, provider TEXT, plan TEXT, budget_usd REAL, reset_day INTEGER, models TEXT);
            CREATE TABLE ipc_budget_log (id INTEGER PRIMARY KEY, subscription TEXT, date TEXT, tokens_in INTEGER, tokens_out INTEGER, estimated_cost_usd REAL, model TEXT, task_ref TEXT);
        ").unwrap();
        conn
    }

    #[test]
    fn test_analyze_task_codegen() {
        let a = analyze_task("implement a new REST endpoint");
        assert_eq!(a.task_type, TaskType::CodeGen);
    }

    #[test]
    fn test_analyze_task_testing() {
        let a = analyze_task("write unit tests for auth module");
        assert_eq!(a.task_type, TaskType::Testing);
    }

    #[test]
    fn test_analyze_task_security() {
        let a = analyze_task("security audit of the API");
        assert_eq!(a.task_type, TaskType::SecurityReview);
    }

    #[test]
    fn test_analyze_task_architecture() {
        let a = analyze_task("design the microservices architecture");
        assert_eq!(a.task_type, TaskType::Architecture);
    }

    #[test]
    fn test_route_task_with_models() {
        let conn = setup_db();
        conn.execute("INSERT INTO ipc_model_registry (host,provider,model,size_gb,quantization,last_seen) VALUES ('local','ollama','codellama',7.0,'Q4','2026-01-01')", []).unwrap();
        let decision = route_task(&conn, "implement a function").unwrap();
        assert!(decision.is_some());
        assert_eq!(decision.unwrap().provider, "ollama");
    }

    #[test]
    fn test_route_task_no_models() {
        let conn = setup_db();
        let decision = route_task(&conn, "do something").unwrap();
        assert!(decision.is_none());
    }

    #[test]
    fn test_fallback_chain_with_ollama() {
        let conn = setup_db();
        conn.execute("INSERT INTO ipc_model_registry (host,provider,model,size_gb,quantization,last_seen) VALUES ('m3','ollama','llama3',4.0,'Q4','2026-01-01')", []).unwrap();
        let chain = fallback_chain(&conn, "").unwrap();
        assert!(!chain.is_empty());
        assert!(chain[0].is_free);
    }

    #[test]
    fn test_fallback_chain_degraded() {
        let conn = setup_db();
        let chain = fallback_chain(&conn, "").unwrap();
        assert_eq!(chain.len(), 1);
        assert!(chain[0].degraded);
    }
}
