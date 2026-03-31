use crate::mcp_server::security::McpError;
use crate::mcp_server::web_search::search_web;
use serde_json::{json, Value};

pub fn handle_create_agent(args: &Value, daemon_url: &str, token: Option<&str>) -> Result<Value, McpError> {
    let name = required(args, "name")?;
    let role = required(args, "role")?;
    let expertise = required(args, "expertise")?;
    let department = required(args, "department")?;
    let org_id = required(args, "org_id")?;
    let query = format!("{role} job description best practices");
    let research = search_web(&query).unwrap_or_default();
    let md = render_agent_markdown(name, role, expertise, department, &research);
    let path = format!("claude-config/agents/{name}.md");
    std::fs::write(&path, md).map_err(|e| McpError::DaemonError(format!("agent file write failed: {e}")))?;
    let client = reqwest::blocking::Client::new();
    post(
        &client,
        &format!("{daemon_url}/api/ipc/agents/register"),
        token,
        &json!({
            "agent_id": name,
            "host": "agent-factory",
            "agent_type": "claude",
            "metadata": json!({"org_id": org_id, "role": role, "department": department}).to_string()
        }),
    )?;
    post(
        &client,
        &format!("{daemon_url}/api/orgs/{org_id}/members"),
        token,
        &json!({"agent": name, "role": role, "department": department}),
    )?;
    let rationale = research
        .first()
        .map(|r| format!("based on {} ({})", r.title, r.url))
        .unwrap_or_else(|| "based on role requirements".to_string());
    post(
        &client,
        &format!("{daemon_url}/api/orgs/{org_id}/decisions"),
        token,
        &json!({
            "decision": format!("Created agent {name} because {rationale}"),
            "rationale": format!("{role} needed for {department} in org {org_id}"),
            "made_by": "ceo",
            "refs": [format!("search:{query}")]
        }),
    )?;
    Ok(json!({"ok": true, "agent_name": name, "agent_file": path, "research_count": research.len()}))
}

fn required<'a>(args: &'a Value, key: &'static str) -> Result<&'a str, McpError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .ok_or(McpError::InvalidParams(key))
}

fn post(client: &reqwest::blocking::Client, url: &str, token: Option<&str>, body: &Value) -> Result<(), McpError> {
    let mut req = client.post(url).json(body);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    let resp = req.send().map_err(|_| McpError::DaemonUnreachable)?;
    if !resp.status().is_success() {
        return Err(McpError::DaemonError(format!("HTTP {} from {url}", resp.status().as_u16())));
    }
    Ok(())
}

pub fn render_agent_markdown(
    name: &str,
    role: &str,
    expertise: &str,
    department: &str,
    research: &[crate::mcp_server::web_search::SearchResult],
) -> String {
    let refs = research
        .iter()
        .take(2)
        .map(|r| format!("- {} ({})", r.title, r.url))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "---\nname: {name}\ndescription: {role} for {department}\nmodel: claude-sonnet-4.6\ntools: [Read, Write, Edit, Bash, WebFetch]\n---\n\n# Role\n{role}\n\n# Expertise\n{expertise}\n\n# Department\n{department}\n\n# Research\n{refs}\n"
    )
}

pub fn register_agent_records(
    conn: &rusqlite::Connection,
    agent: &str,
    org_id: &str,
    role: &str,
    department: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO ipc_agents(name, host, agent_type, registered_at, last_seen)
         VALUES (?1, 'agent-factory', 'claude', 'now', 'now')",
        rusqlite::params![agent],
    )?;
    conn.execute(
        "INSERT INTO ipc_org_members(id, org_id, agent, role, department)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![format!("member-{agent}"), org_id, agent, role, department],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::mcp_server::web_search::SearchResult;

    #[test]
    fn generates_valid_frontmatter() {
        let md = super::render_agent_markdown(
            "iris-recruiter",
            "Talent Partner",
            "Hiring and sourcing",
            "people",
            &[SearchResult {
                title: "Talent Partner Guide".into(),
                snippet: "Best practices".into(),
                url: "https://example.com".into(),
            }],
        );
        assert!(md.starts_with("---\nname: iris-recruiter\n"));
        assert!(md.contains("tools: [Read, Write, Edit, Bash, WebFetch]"));
    }

    #[test]
    fn registers_records_in_db() {
        let conn = rusqlite::Connection::open_in_memory().expect("db");
        conn.execute_batch(
            "CREATE TABLE ipc_agents(name TEXT, host TEXT, agent_type TEXT, registered_at TEXT, last_seen TEXT);
             CREATE TABLE ipc_org_members(id TEXT, org_id TEXT, agent TEXT, role TEXT, department TEXT);",
        )
        .expect("schema");
        super::register_agent_records(&conn, "iris-recruiter", "org-1", "Talent Partner", "people")
            .expect("register");
        let c: i64 = conn.query_row("SELECT COUNT(*) FROM ipc_agents WHERE name='iris-recruiter'", [], |r| r.get(0)).expect("count");
        let m: i64 = conn.query_row("SELECT COUNT(*) FROM ipc_org_members WHERE agent='iris-recruiter' AND org_id='org-1'", [], |r| r.get(0)).expect("count");
        assert_eq!(c, 1);
        assert_eq!(m, 1);
    }
}
