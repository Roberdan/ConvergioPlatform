use serde_json::{json, Value};
use std::time::Duration;

use crate::capabilities::ring::Ring;
use crate::mcp_server::security::McpError;
use crate::mcp_server::tools::McpTool;

pub fn chat_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "cvg_agent_send".into(),
            description: "Send a direct IPC message to another agent.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "to": {"type": "string", "description": "Recipient agent name"},
                    "message": {"type": "string", "description": "Message content"}
                },
                "required": ["to", "message"]
            }),
            min_ring: Ring::Trusted,
        },
        McpTool {
            name: "cvg_agent_ask".into(),
            description: "Send a direct message and wait for reply from another agent.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "to": {"type": "string", "description": "Recipient agent name"},
                    "message": {"type": "string", "description": "Prompt content"},
                    "timeout_secs": {"type": "integer", "description": "Optional timeout in seconds"}
                },
                "required": ["to", "message"]
            }),
            min_ring: Ring::Trusted,
        },
    ]
}

pub fn handle_agent_send(
    daemon_url: &str,
    token: Option<&str>,
    args: &Value,
) -> Result<Value, McpError> {
    let to = args
        .get("to")
        .and_then(|v| v.as_str())
        .ok_or(McpError::InvalidParams("to is required"))?;
    let message = args
        .get("message")
        .and_then(|v| v.as_str())
        .ok_or(McpError::InvalidParams("message is required"))?;
    http_post(
        &format!("{daemon_url}/api/ipc/send-direct"),
        token,
        &json!({"to_agent": to, "content": message}),
    )
}

pub fn handle_agent_ask(
    daemon_url: &str,
    token: Option<&str>,
    args: &Value,
) -> Result<Value, McpError> {
    let to = args
        .get("to")
        .and_then(|v| v.as_str())
        .ok_or(McpError::InvalidParams("to is required"))?;
    let message = args
        .get("message")
        .and_then(|v| v.as_str())
        .ok_or(McpError::InvalidParams("message is required"))?;
    let mut body = json!({"to_agent": to, "content": message});
    if let Some(timeout) = args.get("timeout_secs").and_then(|v| v.as_i64()) {
        body["timeout_secs"] = json!(timeout);
    }
    http_post(&format!("{daemon_url}/api/ipc/ask"), token, &body)
}

fn http_post(url: &str, token: Option<&str>, body: &Value) -> Result<Value, McpError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new());
    let mut req = client.post(url).json(body);
    if let Some(t) = token {
        req = req.bearer_auth(t);
    }
    let resp = req.send().map_err(|_| McpError::DaemonUnreachable)?;
    if !resp.status().is_success() {
        return Err(McpError::DaemonError(format!(
            "HTTP {}",
            resp.status().as_u16()
        )));
    }
    resp.json::<Value>()
        .map_err(|e| McpError::DaemonError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_tools_schema_is_stable() {
        let tools = chat_tools();
        let send = tools
            .iter()
            .find(|t| t.name == "cvg_agent_send")
            .expect("send missing");
        assert_eq!(send.input_schema["required"], json!(["to", "message"]));
        let ask = tools
            .iter()
            .find(|t| t.name == "cvg_agent_ask")
            .expect("ask missing");
        assert_eq!(ask.input_schema["required"], json!(["to", "message"]));
    }

    #[test]
    fn chat_handler_missing_params_returns_expected_shape() {
        let err = handle_agent_ask("http://localhost:8420", None, &json!({"to":"priya"}))
            .expect_err("missing message must fail");
        assert_eq!(err.message(), "Invalid params: message is required");
    }
}
