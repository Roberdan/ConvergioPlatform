use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- T8071: Registration ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkill {
    pub agent: String,
    pub host: String,
    pub skill: String,
    pub confidence: f64,
    pub last_used: String,
}

pub fn register_skills(
    conn: &Connection,
    agent: &str,
    host: &str,
    skills: &[(&str, f64)],
) -> rusqlite::Result<()> {
    for (skill, confidence) in skills {
        conn.execute(
            "INSERT OR REPLACE INTO ipc_agent_skills (agent, host, skill, confidence, last_used)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![agent, host, skill, confidence],
        )?;
    }
    Ok(())
}

pub fn update_skill_usage(
    conn: &Connection,
    agent: &str,
    host: &str,
    skill: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE ipc_agent_skills SET last_used=datetime('now') WHERE agent=?1 AND host=?2 AND skill=?3",
        params![agent, host, skill],
    )?;
    Ok(())
}

pub fn unregister_agent_skills(
    conn: &Connection,
    agent: &str,
    host: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "DELETE FROM ipc_agent_skills WHERE agent=?1 AND host=?2",
        params![agent, host],
    )
}

// --- T8072: Pool queries ---

pub fn get_skill_pool(conn: &Connection) -> rusqlite::Result<HashMap<String, Vec<AgentSkill>>> {
    let mut stmt = conn.prepare(
        "SELECT agent, host, skill, confidence, last_used FROM ipc_agent_skills ORDER BY skill, confidence DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(AgentSkill {
            agent: row.get(0)?,
            host: row.get(1)?,
            skill: row.get(2)?,
            confidence: row.get(3)?,
            last_used: row.get(4)?,
        })
    })?;
    let mut pool: HashMap<String, Vec<AgentSkill>> = HashMap::new();
    for r in rows {
        let skill = r?;
        pool.entry(skill.skill.clone()).or_default().push(skill);
    }
    Ok(pool)
}

pub fn get_agents_for_skill(conn: &Connection, skill: &str) -> rusqlite::Result<Vec<AgentSkill>> {
    let mut stmt = conn.prepare(
        "SELECT agent, host, skill, confidence, last_used FROM ipc_agent_skills
         WHERE skill=?1 ORDER BY confidence DESC",
    )?;
    let rows = stmt.query_map(params![skill], |row| {
        Ok(AgentSkill {
            agent: row.get(0)?,
            host: row.get(1)?,
            skill: row.get(2)?,
            confidence: row.get(3)?,
            last_used: row.get(4)?,
        })
    })?;
    rows.collect()
}

pub fn get_skills_for_agent(conn: &Connection, agent: &str) -> rusqlite::Result<Vec<AgentSkill>> {
    let mut stmt = conn.prepare(
        "SELECT agent, host, skill, confidence, last_used FROM ipc_agent_skills
         WHERE agent=?1 ORDER BY skill",
    )?;
    let rows = stmt.query_map(params![agent], |row| {
        Ok(AgentSkill {
            agent: row.get(0)?,
            host: row.get(1)?,
            skill: row.get(2)?,
            confidence: row.get(3)?,
            last_used: row.get(4)?,
        })
    })?;
    rows.collect()
}

// --- T8074: Request/response protocol ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RequestStatus {
    Pending,
    Assigned,
    InProgress,
    Completed,
    Failed,
}

impl std::fmt::Display for RequestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Assigned => write!(f, "assigned"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillRequest {
    pub id: String,
    pub skill: String,
    pub payload: String,
    pub status: RequestStatus,
    pub assigned_agent: Option<String>,
    pub assigned_host: Option<String>,
    pub result: Option<String>,
    pub created_at: String,
}

pub fn create_skill_request(
    conn: &Connection,
    skill: &str,
    payload: &str,
) -> rusqlite::Result<String> {
    let id = format!("sr-{}", &uuid_v4()[..8]);
    conn.execute(
        "INSERT INTO ipc_agent_skills (agent, host, skill, confidence, last_used)
         SELECT '', '', ?1, 0.0, datetime('now')
         WHERE NOT EXISTS (SELECT 1 FROM ipc_agent_skills WHERE skill=?1 LIMIT 1)",
        params![skill],
    )?;
    // Store request in a lightweight way using session_state
    let req_json = serde_json::json!({
        "id": id, "skill": skill, "payload": payload,
        "status": "pending", "created_at": chrono_now()
    });
    conn.execute(
        "INSERT OR REPLACE INTO session_state (key, value) VALUES (?1, ?2)",
        params![format!("skill_req:{id}"), req_json.to_string()],
    )?;
    Ok(id)
}

pub fn find_best_agent(
    conn: &Connection,
    skill: &str,
) -> rusqlite::Result<Option<(String, String)>> {
    let result = conn.query_row(
        "SELECT agent, host FROM ipc_agent_skills
         WHERE skill=?1 AND agent != '' ORDER BY confidence DESC LIMIT 1",
        params![skill],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    );
    match result {
        Ok(r) => Ok(Some(r)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn assign_request(
    conn: &Connection,
    request_id: &str,
    agent: &str,
    host: &str,
) -> rusqlite::Result<()> {
    let key = format!("skill_req:{request_id}");
    let val: String = conn.query_row(
        "SELECT value FROM session_state WHERE key=?1",
        params![key],
        |r| r.get(0),
    )?;
    let mut req: serde_json::Value = serde_json::from_str(&val).unwrap_or_default();
    req["status"] = "assigned".into();
    req["assigned_agent"] = agent.into();
    req["assigned_host"] = host.into();
    conn.execute(
        "UPDATE session_state SET value=?1 WHERE key=?2",
        params![req.to_string(), key],
    )?;
    Ok(())
}

// --- T8076: Response handling ---

pub fn complete_skill_request(
    conn: &Connection,
    request_id: &str,
    result: &str,
) -> rusqlite::Result<()> {
    let key = format!("skill_req:{request_id}");
    let val: String = conn.query_row(
        "SELECT value FROM session_state WHERE key=?1",
        params![key],
        |r| r.get(0),
    )?;
    let mut req: serde_json::Value = serde_json::from_str(&val).unwrap_or_default();
    req["status"] = "completed".into();
    req["result"] = result.into();
    conn.execute(
        "UPDATE session_state SET value=?1 WHERE key=?2",
        params![req.to_string(), key],
    )?;
    Ok(())
}

pub fn get_request_result(conn: &Connection, request_id: &str) -> rusqlite::Result<Option<String>> {
    let key = format!("skill_req:{request_id}");
    let val: String = conn.query_row(
        "SELECT value FROM session_state WHERE key=?1",
        params![key],
        |r| r.get(0),
    )?;
    let req: serde_json::Value = serde_json::from_str(&val).unwrap_or_default();
    Ok(req["result"].as_str().map(String::from))
}

pub fn fail_skill_request(
    conn: &Connection,
    request_id: &str,
    reason: &str,
) -> rusqlite::Result<()> {
    let key = format!("skill_req:{request_id}");
    let val: String = conn.query_row(
        "SELECT value FROM session_state WHERE key=?1",
        params![key],
        |r| r.get(0),
    )?;
    let mut req: serde_json::Value = serde_json::from_str(&val).unwrap_or_default();
    req["status"] = "failed".into();
    req["result"] = reason.into();
    conn.execute(
        "UPDATE session_state SET value=?1 WHERE key=?2",
        params![req.to_string(), key],
    )?;
    Ok(())
}

// --- T8077: Rating ---

pub fn rate_skill_response(
    conn: &Connection,
    request_id: &str,
    rating: f64,
) -> rusqlite::Result<()> {
    let key = format!("skill_req:{request_id}");
    let val: String = conn.query_row(
        "SELECT value FROM session_state WHERE key=?1",
        params![key],
        |r| r.get(0),
    )?;
    let req: serde_json::Value = serde_json::from_str(&val).unwrap_or_default();
    let skill = req["skill"].as_str().unwrap_or("");
    let agent = req["assigned_agent"].as_str().unwrap_or("");
    let host = req["assigned_host"].as_str().unwrap_or("");
    if !agent.is_empty() && !skill.is_empty() {
        // Weighted moving average: 80% old + 20% new
        let current: f64 = conn
            .query_row(
                "SELECT confidence FROM ipc_agent_skills WHERE agent=?1 AND host=?2 AND skill=?3",
                params![agent, host, skill],
                |r| r.get(0),
            )
            .unwrap_or(0.5);
        let moving_avg = current * 0.8 + rating * 0.2;
        conn.execute(
            "UPDATE ipc_agent_skills SET confidence=?1 WHERE agent=?2 AND host=?3 AND skill=?4",
            params![moving_avg, agent, host, skill],
        )?;
    }
    Ok(())
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let pid = std::process::id();
    format!("{:016x}{:08x}", ts.as_nanos(), pid)
}

fn chrono_now() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{ts}")
}

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
