use super::state::{query_rows, ServerState};
use super::ws_brain::broadcast_brain_event;
use serde_json::json;

pub fn broadcast_org_update(state: &ServerState, org_id: &str, action: &str) {
    broadcast_brain_event(state, "org_update", json!({ "org_id": org_id, "action": action }));
}

pub fn broadcast_org_message(state: &ServerState, channel: &str, from: &str, content: &str) {
    let org_id = channel.strip_prefix("org:").unwrap_or(channel);
    broadcast_brain_event(
        state,
        "org_message",
        json!({ "org_id": org_id, "channel": channel, "from": from, "content": content }),
    );
}

pub fn broadcast_agent_factory(state: &ServerState, org_id: &str, agent: &str, role: &str) {
    broadcast_brain_event(
        state,
        "agent_factory",
        json!({ "org_id": org_id, "agent": agent, "role": role }),
    );
}

pub fn broadcast_org_topology(state: &ServerState) {
    let payload = match state.get_conn() {
        Ok(conn) => {
            let orgs = query_rows(&conn, "SELECT id, status, ceo_agent FROM ipc_orgs ORDER BY id", [])
                .unwrap_or_default();
            let links = query_rows(
                &conn,
                "SELECT substr(channel, 11, instr(substr(channel, 11), ':') - 1) AS source_org,
                        substr(channel, 11 + instr(substr(channel, 11), ':')) AS target_org,
                        COUNT(*) AS volume
                 FROM ipc_messages
                 WHERE channel LIKE 'inter-org:%:%'
                 GROUP BY source_org, target_org",
                [],
            )
            .unwrap_or_default();
            json!({ "orgs": orgs, "links": links })
        }
        Err(_) => json!({ "orgs": [], "links": [] }),
    };
    broadcast_brain_event(state, "org_topology", payload);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn test_db_path(prefix: &str) -> PathBuf {
        static CTR: AtomicU64 = AtomicU64::new(0);
        let n = CTR.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("{prefix}-{}-{n}.db", std::process::id()))
    }

    #[test]
    fn org_update_event_format_is_emitted() {
        let state = ServerState::new(test_db_path("test-ws-org"), None);
        let mut rx = state.ws_tx.subscribe();
        broadcast_org_update(&state, "org-a", "updated");
        let event = rx.try_recv().expect("event");
        assert_eq!(event["event_type"], "org_update");
        assert_eq!(event["payload"]["org_id"], "org-a");
    }

    #[test]
    fn topology_event_includes_inter_org_links() {
        let state = ServerState::new(test_db_path("test-ws-org-topology"), None);
        let conn = state.get_conn().expect("conn");
        conn.execute_batch("CREATE TABLE IF NOT EXISTS ipc_orgs(id TEXT, status TEXT, ceo_agent TEXT);")
            .expect("schema");
        conn.execute("INSERT INTO ipc_orgs(id,status,ceo_agent) VALUES ('org-a','active','ceo-a')", [])
            .expect("org a");
        conn.execute("INSERT INTO ipc_orgs(id,status,ceo_agent) VALUES ('org-b','active','ceo-b')", [])
            .expect("org b");
        conn.execute(
            "INSERT INTO ipc_messages(id, from_agent, channel, content)
             VALUES ('msg-1', 'ceo-a', 'inter-org:org-a:org-b', 'sync request')",
            [],
        )
            .expect("msg");
        drop(conn);

        let mut rx = state.ws_tx.subscribe();
        broadcast_org_topology(&state);
        let event = rx.try_recv().expect("event");
        assert_eq!(event["event_type"], "org_topology");
        assert_eq!(event["payload"]["links"][0]["source_org"], "org-a");
        assert_eq!(event["payload"]["links"][0]["target_org"], "org-b");
    }
}
