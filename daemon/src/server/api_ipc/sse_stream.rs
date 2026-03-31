use super::super::state::ServerState;
use axum::extract::{Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use chrono::Utc;
use futures_util::stream::unfold;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::convert::Infallible;
use tokio::sync::broadcast::error::RecvError;

pub async fn api_ipc_stream(
    State(state): State<ServerState>,
    Query(qs): Query<HashMap<String, String>>,
) -> Sse<impl futures_util::stream::Stream<Item = Result<Event, Infallible>>> {
    let agent = qs.get("agent").cloned().unwrap_or_default();
    let rx = state.ws_tx.subscribe();
    let stream = unfold((rx, agent), |(mut rx, agent)| async move {
        let next = loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Some(data) = build_event_data(&event, &agent) {
                        break Ok(Event::default().data(data));
                    }
                }
                Err(RecvError::Lagged(dropped)) => {
                    let event = Event::default()
                        .event("reconnect")
                        .data(lagged_reconnect_hint(dropped));
                    break Ok(event);
                }
                Err(RecvError::Closed) => return None,
            }
        };
        Some((next, (rx, agent)))
    });

    Sse::new(stream).keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(15)))
}

pub fn build_event_data(event: &Value, agent: &str) -> Option<String> {
    if event.get("type").and_then(Value::as_str) != Some("ipc_direct_message") {
        return None;
    }
    let from = event.get("from")?.as_str()?;
    let to = event.get("to")?.as_str()?;
    let content = event.get("content")?.as_str()?;
    if !agent.is_empty() && agent != from && agent != to {
        return None;
    }
    Some(
        json!({
            "from": from,
            "to": to,
            "content": content,
            "ts": Utc::now().to_rfc3339(),
        })
        .to_string(),
    )
}

pub fn lagged_reconnect_hint(dropped: u64) -> String {
    json!({
        "reconnect": true,
        "reason": "lagged",
        "dropped": dropped
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sse_stream_event_format() {
        let event = json!({
            "type": "ipc_direct_message",
            "from": "priya",
            "to": "roberto",
            "content": "ciao"
        });
        let data = build_event_data(&event, "roberto").expect("message for roberto");
        let parsed: Value = serde_json::from_str(&data).expect("valid json");
        assert_eq!(parsed["from"], "priya");
        assert_eq!(parsed["to"], "roberto");
        assert_eq!(parsed["content"], "ciao");
        assert!(parsed["ts"].as_str().is_some());
    }

    #[test]
    fn sse_stream_backpressure_lag_detection() {
        let data = lagged_reconnect_hint(7);
        let parsed: Value = serde_json::from_str(&data).expect("valid json");
        assert_eq!(parsed["reconnect"], true);
        assert_eq!(parsed["reason"], "lagged");
        assert_eq!(parsed["dropped"], 7);
    }
}
