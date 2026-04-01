// Brain org enrichment: org summaries + agent relations for /api/brain.

use rusqlite::Connection;
use serde_json::{json, Value};

/// Enrich brain response with org-level summaries: health, agents, budget.
pub fn enrich_brain_with_orgs(conn: &Connection) -> Value {
    let orgs: Vec<Value> = conn
        .prepare(
            "SELECT o.id, o.mission, o.ceo_agent, o.budget, o.status,
                    o.daily_budget_tokens
             FROM ipc_orgs o ORDER BY o.id",
        )
        .and_then(|mut stmt| {
            let rows = stmt.query_map([], |row| {
                let slug: String = row.get(0)?;
                let mission: String = row.get(1)?;
                let ceo: String = row.get(2)?;
                let budget: f64 = row.get(3)?;
                let status: String = row.get(4)?;
                let daily_tokens: i64 = row.get(5)?;
                Ok((slug, mission, ceo, budget, status, daily_tokens))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .unwrap_or_default()
        .into_iter()
        .map(|(slug, _mission, ceo, budget, status, _daily)| {
            let active_agents = active_agents_count(conn, &slug);
            let active_plans = active_plans_count(conn, &slug);
            let budget_used = budget_used_sum(conn, &slug);
            let last_decision = last_decision_text(conn, &slug);
            let health = calc_health(budget, budget_used, active_agents);
            json!({
                "slug": slug,
                "status": status,
                "ceo_agent": ceo,
                "active_agents": active_agents,
                "active_plans": active_plans,
                "budget_used": budget_used,
                "budget_total": budget,
                "last_decision": last_decision,
                "health": health,
            })
        })
        .collect();
    Value::Array(orgs)
}

fn active_agents_count(conn: &Connection, org_id: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM ipc_org_members m
         JOIN ipc_agents a ON m.agent = a.name
         WHERE m.org_id = ?1
           AND a.last_seen >= strftime('%Y-%m-%dT%H:%M:%f','now','-10 minutes')",
        rusqlite::params![org_id],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

fn active_plans_count(conn: &Connection, org_id: &str) -> i64 {
    // plans.org_id may not exist yet; fall back to 0
    conn.query_row(
        "SELECT COUNT(*) FROM plans WHERE org_id = ?1 AND status = 'doing'",
        rusqlite::params![org_id],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

fn budget_used_sum(conn: &Connection, org_id: &str) -> f64 {
    conn.query_row(
        "SELECT COALESCE(SUM(value), 0.0) FROM ipc_org_telemetry
         WHERE org_id = ?1 AND metric = 'cost'",
        rusqlite::params![org_id],
        |r| r.get(0),
    )
    .unwrap_or(0.0)
}

fn last_decision_text(conn: &Connection, org_id: &str) -> Option<String> {
    conn.query_row(
        "SELECT decision FROM ipc_decisions
         WHERE org_id = ?1 ORDER BY created_at DESC LIMIT 1",
        rusqlite::params![org_id],
        |r| r.get(0),
    )
    .ok()
}

fn calc_health(budget: f64, used: f64, active_agents: i64) -> &'static str {
    if active_agents == 0 || (budget > 0.0 && used / budget > 0.8) {
        "red"
    } else if budget > 0.0 && used / budget > 0.5 {
        "yellow"
    } else {
        "green"
    }
}

/// Agent-to-agent message relations from the last hour.
pub fn query_agent_relations(conn: &Connection) -> Value {
    let rows: Vec<Value> = conn
        .prepare(
            "SELECT from_agent, to_agent, COUNT(*) AS message_count,
                    MAX(created_at) AS last_at
             FROM ipc_messages
             WHERE created_at >= strftime('%Y-%m-%dT%H:%M:%f','now','-1 hour')
               AND from_agent != '' AND to_agent IS NOT NULL AND to_agent != ''
             GROUP BY from_agent, to_agent
             ORDER BY message_count DESC",
        )
        .and_then(|mut stmt| {
            let mapped = stmt.query_map([], |row| {
                Ok(json!({
                    "from": row.get::<_, String>(0)?,
                    "to": row.get::<_, String>(1)?,
                    "message_count": row.get::<_, i64>(2)?,
                    "last_at": row.get::<_, String>(3)?,
                }))
            })?;
            mapped.collect::<rusqlite::Result<Vec<_>>>()
        })
        .unwrap_or_default();
    Value::Array(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn test_conn(prefix: &str) -> Connection {
        static CTR: AtomicU64 = AtomicU64::new(0);
        let n = CTR.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir()
            .join(format!("{prefix}-{}-{n}.db", std::process::id()));
        let conn = Connection::open(p).expect("open");
        conn.execute_batch(SCHEMA).expect("schema");
        conn
    }

    const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS ipc_orgs (
    id TEXT PRIMARY KEY, mission TEXT, ceo_agent TEXT,
    budget REAL DEFAULT 0, status TEXT DEFAULT 'active',
    daily_budget_tokens INTEGER DEFAULT 1000
);
CREATE TABLE IF NOT EXISTS ipc_org_members (
    id TEXT PRIMARY KEY, org_id TEXT, agent TEXT, role TEXT, department TEXT
);
CREATE TABLE IF NOT EXISTS ipc_agents (
    name TEXT PRIMARY KEY, host TEXT, agent_type TEXT,
    last_seen TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now'))
);
CREATE TABLE IF NOT EXISTS ipc_decisions (
    id TEXT PRIMARY KEY, org_id TEXT, decision TEXT,
    rationale TEXT, decided_by TEXT, created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now'))
);
CREATE TABLE IF NOT EXISTS ipc_org_telemetry (
    id TEXT PRIMARY KEY, org_id TEXT, metric TEXT,
    value REAL, tags TEXT, created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now'))
);
CREATE TABLE IF NOT EXISTS ipc_messages (
    id TEXT PRIMARY KEY, from_agent TEXT, to_agent TEXT,
    channel TEXT, content TEXT, created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now'))
);
CREATE TABLE IF NOT EXISTS plans (
    id INTEGER PRIMARY KEY, name TEXT, status TEXT, org_id TEXT
);
";

    fn seed(conn: &Connection) {
        conn.execute_batch("
INSERT INTO ipc_orgs(id, mission, ceo_agent, budget, status)
  VALUES ('acme', 'Build widgets', 'ceo-acme', 10.0, 'active');
INSERT INTO ipc_agents(name, host, agent_type)
  VALUES ('agent-1', 'local', 'copilot');
INSERT INTO ipc_org_members(id, org_id, agent, role)
  VALUES ('m1', 'acme', 'agent-1', 'engineer');
INSERT INTO ipc_org_telemetry(id, org_id, metric, value)
  VALUES ('t1', 'acme', 'cost', 3.0);
INSERT INTO ipc_decisions(id, org_id, decision, decided_by)
  VALUES ('d1', 'acme', 'Ship v2', 'ceo-acme');
INSERT INTO plans(id, name, status, org_id)
  VALUES (1, 'Alpha', 'doing', 'acme');
INSERT INTO ipc_messages(id, from_agent, to_agent, content)
  VALUES ('msg1', 'agent-1', 'ceo-acme', 'ready');
        ").expect("seed");
    }

    #[test]
    fn enrich_returns_org_shape() {
        let conn = test_conn("brain-org-enrich");
        seed(&conn);
        let orgs = enrich_brain_with_orgs(&conn);
        let arr = orgs.as_array().expect("array");
        assert_eq!(arr.len(), 1);
        let org = &arr[0];
        assert_eq!(org["slug"], "acme");
        assert_eq!(org["ceo_agent"], "ceo-acme");
        assert_eq!(org["active_agents"], 1);
        assert_eq!(org["active_plans"], 1);
        assert_eq!(org["budget_total"], 10.0);
        assert_eq!(org["budget_used"], 3.0);
        assert_eq!(org["last_decision"], "Ship v2");
        assert_eq!(org["health"], "green");
    }

    #[test]
    fn health_red_when_no_active_agents() {
        let conn = test_conn("brain-org-health-red");
        conn.execute(
            "INSERT INTO ipc_orgs(id, mission, ceo_agent, budget)
             VALUES ('empty', 'None', 'nobody', 5.0)",
            [],
        ).unwrap();
        let orgs = enrich_brain_with_orgs(&conn);
        assert_eq!(orgs[0]["health"], "red");
    }

    #[test]
    fn agent_relations_groups_messages() {
        let conn = test_conn("brain-org-relations");
        seed(&conn);
        let rels = query_agent_relations(&conn);
        let arr = rels.as_array().expect("array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["from"], "agent-1");
        assert_eq!(arr[0]["to"], "ceo-acme");
        assert_eq!(arr[0]["message_count"], 1);
        assert!(arr[0]["last_at"].is_string());
    }

    #[test]
    fn empty_db_returns_empty_arrays() {
        let conn = test_conn("brain-org-empty");
        let orgs = enrich_brain_with_orgs(&conn);
        assert_eq!(orgs.as_array().unwrap().len(), 0);
        let rels = query_agent_relations(&conn);
        assert_eq!(rels.as_array().unwrap().len(), 0);
    }
}
