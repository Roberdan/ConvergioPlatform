pub mod executor;
pub mod registry;

pub use executor::{
    assign_request, complete_skill_request, create_skill_request, fail_skill_request,
    find_best_agent, get_request_result, rate_skill_response, RequestStatus, SkillRequest,
};
pub use registry::{
    get_agents_for_skill, get_skill_pool, get_skills_for_agent, register_skills,
    unregister_agent_skills, update_skill_usage, AgentSkill,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("
            CREATE TABLE ipc_agent_skills (id INTEGER PRIMARY KEY, agent TEXT, host TEXT, skill TEXT, confidence REAL DEFAULT 0.5, last_used TEXT, UNIQUE(agent,host,skill));
            CREATE TABLE session_state (key TEXT PRIMARY KEY, value TEXT);
        ").unwrap();
        conn
    }

    #[test]
    fn test_register_and_pool() {
        let conn = setup_db();
        register_skills(&conn, "agent1", "m3", &[("rust", 0.9), ("python", 0.7)]).unwrap();
        let pool = get_skill_pool(&conn).unwrap();
        assert_eq!(pool.len(), 2);
        assert!(pool.contains_key("rust"));
    }

    #[test]
    fn test_get_agents_for_skill() {
        let conn = setup_db();
        register_skills(&conn, "a1", "h1", &[("coding", 0.9)]).unwrap();
        register_skills(&conn, "a2", "h2", &[("coding", 0.5)]).unwrap();
        let agents = get_agents_for_skill(&conn, "coding").unwrap();
        assert_eq!(agents.len(), 2);
        assert!(agents[0].confidence >= agents[1].confidence);
    }

    #[test]
    fn test_find_best_agent() {
        let conn = setup_db();
        register_skills(&conn, "low", "h", &[("review", 0.3)]).unwrap();
        register_skills(&conn, "high", "h", &[("review", 0.9)]).unwrap();
        let best = find_best_agent(&conn, "review").unwrap().unwrap();
        assert_eq!(best.0, "high");
    }

    #[test]
    fn test_request_lifecycle() {
        let conn = setup_db();
        register_skills(&conn, "worker", "h1", &[("debug", 0.8)]).unwrap();
        let id = create_skill_request(&conn, "debug", "fix this bug").unwrap();
        assert!(id.starts_with("sr-"));
        assign_request(&conn, &id, "worker", "h1").unwrap();
        complete_skill_request(&conn, &id, "fixed it").unwrap();
        let result = get_request_result(&conn, &id).unwrap();
        assert_eq!(result, Some("fixed it".to_string()));
    }

    #[test]
    fn test_rate_skill_weighted_avg() {
        let conn = setup_db();
        register_skills(&conn, "a", "h", &[("test", 0.5)]).unwrap();
        let id = create_skill_request(&conn, "test", "payload").unwrap();
        assign_request(&conn, &id, "a", "h").unwrap();
        complete_skill_request(&conn, &id, "done").unwrap();
        // 0.8 * 0.5 + 0.2 * 1.0 = 0.6
        rate_skill_response(&conn, &id, 1.0).unwrap();
        let skills = get_skills_for_agent(&conn, "a").unwrap();
        let conf = skills
            .iter()
            .find(|s| s.skill == "test")
            .unwrap()
            .confidence;
        assert!((conf - 0.6).abs() < 0.01, "expected ~0.6, got {conf}");
    }

    #[test]
    fn test_fail_request() {
        let conn = setup_db();
        let id = create_skill_request(&conn, "x", "y").unwrap();
        fail_skill_request(&conn, &id, "timeout").unwrap();
        let result = get_request_result(&conn, &id).unwrap();
        assert_eq!(result, Some("timeout".to_string()));
    }

    #[test]
    fn test_unregister() {
        let conn = setup_db();
        register_skills(&conn, "a", "h", &[("s1", 0.5), ("s2", 0.5)]).unwrap();
        let n = unregister_agent_skills(&conn, "a", "h").unwrap();
        assert_eq!(n, 2);
    }
}
