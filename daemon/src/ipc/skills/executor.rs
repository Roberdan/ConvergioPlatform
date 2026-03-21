use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

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
