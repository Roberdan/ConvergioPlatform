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

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn setup_db() -> rusqlite::Result<rusqlite::Connection> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE ipc_agent_skills (id INTEGER PRIMARY KEY, agent TEXT, host TEXT, \
             skill TEXT, confidence REAL DEFAULT 0.5, last_used TEXT, UNIQUE(agent,host,skill)); \
             CREATE TABLE session_state (key TEXT PRIMARY KEY, value TEXT);",
        )?;
        Ok(conn)
    }

    #[test]
    fn test_register_and_pool() -> TestResult {
        let conn = setup_db()?;
        register_skills(&conn, "agent1", "m3", &[("rust", 0.9), ("python", 0.7)])?;
        let pool = get_skill_pool(&conn)?;
        assert_eq!(pool.len(), 2);
        assert!(pool.contains_key("rust"));
        Ok(())
    }

    #[test]
    fn test_get_agents_for_skill() -> TestResult {
        let conn = setup_db()?;
        register_skills(&conn, "a1", "h1", &[("coding", 0.9)])?;
        register_skills(&conn, "a2", "h2", &[("coding", 0.5)])?;
        let agents = get_agents_for_skill(&conn, "coding")?;
        assert_eq!(agents.len(), 2);
        assert!(agents[0].confidence >= agents[1].confidence);
        Ok(())
    }

    #[test]
    fn test_find_best_agent() -> TestResult {
        let conn = setup_db()?;
        register_skills(&conn, "low", "h", &[("review", 0.3)])?;
        register_skills(&conn, "high", "h", &[("review", 0.9)])?;
        let best = find_best_agent(&conn, "review")?.ok_or("no agent found")?;
        assert_eq!(best.0, "high");
        Ok(())
    }

    #[test]
    fn test_request_lifecycle() -> TestResult {
        let conn = setup_db()?;
        register_skills(&conn, "worker", "h1", &[("debug", 0.8)])?;
        let id = create_skill_request(&conn, "debug", "fix this bug")?;
        assert!(id.starts_with("sr-"));
        assign_request(&conn, &id, "worker", "h1")?;
        complete_skill_request(&conn, &id, "fixed it")?;
        let result = get_request_result(&conn, &id)?;
        assert_eq!(result, Some("fixed it".to_string()));
        Ok(())
    }

    #[test]
    fn test_rate_skill_weighted_avg() -> TestResult {
        let conn = setup_db()?;
        register_skills(&conn, "a", "h", &[("test", 0.5)])?;
        let id = create_skill_request(&conn, "test", "payload")?;
        assign_request(&conn, &id, "a", "h")?;
        complete_skill_request(&conn, &id, "done")?;
        // 0.8 * 0.5 + 0.2 * 1.0 = 0.6
        rate_skill_response(&conn, &id, 1.0)?;
        let skills = get_skills_for_agent(&conn, "a")?;
        let conf = skills
            .iter()
            .find(|s| s.skill == "test")
            .ok_or("skill 'test' not found")?
            .confidence;
        assert!((conf - 0.6).abs() < 0.01, "expected ~0.6, got {conf}");
        Ok(())
    }

    #[test]
    fn test_fail_request() -> TestResult {
        let conn = setup_db()?;
        let id = create_skill_request(&conn, "x", "y")?;
        fail_skill_request(&conn, &id, "timeout")?;
        let result = get_request_result(&conn, &id)?;
        assert_eq!(result, Some("timeout".to_string()));
        Ok(())
    }

    #[test]
    fn test_unregister() -> TestResult {
        let conn = setup_db()?;
        register_skills(&conn, "a", "h", &[("s1", 0.5), ("s2", 0.5)])?;
        let n = unregister_agent_skills(&conn, "a", "h")?;
        assert_eq!(n, 2);
        Ok(())
    }
}
