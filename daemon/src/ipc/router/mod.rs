pub mod dispatch;
pub mod handlers;

pub use dispatch::{analyze_task, fallback_chain, route_task, FallbackOption, RouteDecision, TaskAnalysis, TaskType};
pub use handlers::{plan_parallel_execution, ExecutionPlan, SubtaskAssignment};

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
        conn.execute(
            "INSERT INTO ipc_model_registry (host,provider,model,size_gb,quantization,last_seen) VALUES ('local','ollama','codellama',7.0,'Q4','2026-01-01')",
            [],
        ).unwrap();
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
        conn.execute(
            "INSERT INTO ipc_model_registry (host,provider,model,size_gb,quantization,last_seen) VALUES ('m3','ollama','llama3',4.0,'Q4','2026-01-01')",
            [],
        ).unwrap();
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
