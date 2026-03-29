use super::*;
use std::time::Duration;

#[test]
fn test_parse_agent_update() {
    let json = r#"{"kind":"brain_event","event_type":"agent_update","payload":[{"id":1,"name":"thor"}]}"#;
    let event = WsClient::parse_message(json).unwrap();
    match event {
        BrainEvent::AgentUpdate { agents } => {
            assert_eq!(agents.len(), 1);
            assert_eq!(agents[0]["name"], "thor");
        }
        _ => panic!("expected AgentUpdate"),
    }
}

#[test]
fn test_parse_session_update() {
    let json = r#"{"kind":"brain_event","event_type":"session_update","payload":[{"session_id":"abc"}]}"#;
    let event = WsClient::parse_message(json).unwrap();
    match event {
        BrainEvent::SessionUpdate { sessions } => {
            assert_eq!(sessions.len(), 1);
            assert_eq!(sessions[0]["session_id"], "abc");
        }
        _ => panic!("expected SessionUpdate"),
    }
}

#[test]
fn test_parse_task_update() {
    let json = r#"{"kind":"brain_event","event_type":"task_update","payload":{"task_id":42,"status":"done","plan_id":708}}"#;
    let event = WsClient::parse_message(json).unwrap();
    match event {
        BrainEvent::TaskUpdate { task_id, status, plan_id } => {
            assert_eq!(task_id, 42);
            assert_eq!(status, "done");
            assert_eq!(plan_id, 708);
        }
        _ => panic!("expected TaskUpdate"),
    }
}

#[test]
fn test_parse_heartbeat_event() {
    // kind=brain_event + event_type=heartbeat → Heartbeat variant
    let json = r#"{"kind":"brain_event","event_type":"heartbeat"}"#;
    let event = WsClient::parse_message(json).unwrap();
    assert_eq!(event, BrainEvent::Heartbeat);
}

#[test]
fn test_parse_heartbeat_snapshot_with_peers() {
    let json = r#"{"kind":"heartbeat_snapshot","peers":[{"node":"mac-1","status":"ok"},{"node":"linux-2","status":"ok"}]}"#;
    let event = WsClient::parse_message(json).unwrap();
    match event {
        BrainEvent::HeartbeatSnapshot { peers } => {
            assert_eq!(peers.len(), 2);
            assert_eq!(peers[0]["node"], "mac-1");
            assert_eq!(peers[1]["node"], "linux-2");
        }
        _ => panic!("expected HeartbeatSnapshot"),
    }
}

#[test]
fn test_parse_heartbeat_snapshot_empty_peers() {
    // Server may send empty peers array on first connect before any heartbeat
    let json = r#"{"kind":"heartbeat_snapshot","peers":[]}"#;
    let event = WsClient::parse_message(json).unwrap();
    match event {
        BrainEvent::HeartbeatSnapshot { peers } => assert!(peers.is_empty()),
        _ => panic!("expected HeartbeatSnapshot"),
    }
}

#[test]
fn test_parse_heartbeat_snapshot_missing_peers_defaults_empty() {
    let json = r#"{"kind":"heartbeat_snapshot"}"#;
    let event = WsClient::parse_message(json).unwrap();
    match event {
        BrainEvent::HeartbeatSnapshot { peers } => assert!(peers.is_empty()),
        _ => panic!("expected HeartbeatSnapshot"),
    }
}

#[test]
fn test_parse_heartbeat_none() {
    // kind != brain_event and kind != heartbeat_snapshot → None
    let json = r#"{"kind":"heartbeat","event_type":"ping"}"#;
    assert!(WsClient::parse_message(json).is_none());
}

#[test]
fn test_parse_invalid() {
    assert!(WsClient::parse_message("not json at all").is_none());
    assert!(WsClient::parse_message("{}").is_none());
}

#[test]
fn test_backoff_duration() {
    let mut c = WsClient::new("http://localhost:8420");
    assert_eq!(c.backoff_duration(), Duration::from_secs(1)); // 2^0
    c.retry_count = 1;
    assert_eq!(c.backoff_duration(), Duration::from_secs(2)); // 2^1
    c.retry_count = 2;
    assert_eq!(c.backoff_duration(), Duration::from_secs(4)); // 2^2
    c.retry_count = 5;
    assert_eq!(c.backoff_duration(), Duration::from_secs(30)); // capped
    c.retry_count = 10;
    assert_eq!(c.backoff_duration(), Duration::from_secs(30)); // still capped
}

#[test]
fn test_should_fallback_after_3() {
    let mut c = WsClient::new("http://localhost:8420");
    assert!(!c.should_fallback());
    c.increment_retries();
    c.increment_retries();
    assert!(!c.should_fallback());
    c.increment_retries();
    assert!(c.should_fallback()); // 3 >= 3
    c.reset_retries();
    assert!(!c.should_fallback());
}

#[test]
fn test_url_conversion() {
    let c = WsClient::new("http://localhost:8420");
    assert_eq!(c.url, "ws://localhost:8420/ws/brain");

    let c2 = WsClient::new("https://convergio.example.com");
    assert_eq!(c2.url, "wss://convergio.example.com/ws/brain");
}
