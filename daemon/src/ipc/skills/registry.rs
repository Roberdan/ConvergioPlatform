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
