pub mod serialization;
pub mod types;

pub use serialization::{
    add_subscription, advertise_capabilities, get_all_capabilities, health_check_providers,
    list_subscriptions, remove_subscription, start_model_probe, ProviderHealth, Subscription,
};
pub use types::{
    get_all_models, probe_lmstudio, probe_ollama, store_models, ModelEntry, NodeCapabilities,
    OllamaModel,
};

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
