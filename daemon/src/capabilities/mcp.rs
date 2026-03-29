use super::types::{CapabilityError, ToolSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

/// MCP (Model Context Protocol) connector.
/// Supports stdio transport for local MCP servers.
pub struct McpConnector {
    server_cmd: String,
    server_args: Vec<String>,
    child: Option<Child>,
    request_id: u64,
}

/// JSON-RPC request for MCP protocol.
#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

/// JSON-RPC response from MCP server.
#[derive(Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u64,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    #[allow(dead_code)]
    code: i64,
    message: String,
}

impl McpConnector {
    pub fn new(server_cmd: &str, args: &[&str]) -> Self {
        Self {
            server_cmd: server_cmd.to_string(),
            server_args: args.iter().map(|s| s.to_string()).collect(),
            child: None,
            request_id: 0,
        }
    }

    /// Start the MCP server process and send initialize.
    pub fn connect(&mut self) -> Result<Value, CapabilityError> {
        let child = Command::new(&self.server_cmd)
            .args(&self.server_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| CapabilityError::InvocationFailed(format!("spawn MCP server: {e}")))?;
        self.child = Some(child);
        self.send_request("initialize", Some(json!({"capabilities": {}})))
    }

    /// Discover available tools via tools/list.
    pub fn list_tools(&mut self) -> Result<Vec<ToolSchema>, CapabilityError> {
        let resp = self.send_request("tools/list", None)?;
        let tools = resp
            .get("tools")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(tools
            .into_iter()
            .filter_map(|t| {
                Some(ToolSchema {
                    name: t.get("name")?.as_str()?.to_string(),
                    description: t
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("")
                        .to_string(),
                    input_schema: t.get("inputSchema").cloned().unwrap_or(json!({})),
                    output_schema: json!({}),
                })
            })
            .collect())
    }

    /// Invoke a tool via tools/call.
    pub fn invoke(&mut self, tool_name: &str, input: Value) -> Result<Value, CapabilityError> {
        self.send_request(
            "tools/call",
            Some(json!({"name": tool_name, "arguments": input})),
        )
    }

    /// Disconnect: kill the MCP server process.
    pub fn disconnect(&mut self) {
        if let Some(ref mut child) = self.child {
            if let Err(e) = child.kill() {
                tracing::debug!("mcp: kill: {e}");
            }
            if let Err(e) = child.wait() {
                tracing::debug!("mcp: wait: {e}");
            }
        }
        self.child = None;
    }

    /// Send a JSON-RPC request and read the response.
    fn send_request(&mut self, method: &str, params: Option<Value>) -> Result<Value, CapabilityError> {
        self.request_id += 1;
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self.request_id,
            method: method.to_string(),
            params,
        };
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| CapabilityError::InvocationFailed("not connected".to_string()))?;

        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| CapabilityError::InvocationFailed("no stdin".to_string()))?;

        let payload = serde_json::to_string(&req)
            .map_err(|e| CapabilityError::InvocationFailed(format!("serialize: {e}")))?;
        writeln!(stdin, "{payload}")
            .map_err(|e| CapabilityError::InvocationFailed(format!("write: {e}")))?;
        stdin
            .flush()
            .map_err(|e| CapabilityError::InvocationFailed(format!("flush: {e}")))?;

        let stdout = child
            .stdout
            .as_mut()
            .ok_or_else(|| CapabilityError::InvocationFailed("no stdout".to_string()))?;

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|e| CapabilityError::InvocationFailed(format!("read: {e}")))?;

        let resp: JsonRpcResponse = serde_json::from_str(&line)
            .map_err(|e| CapabilityError::InvocationFailed(format!("parse response: {e}")))?;

        if let Some(err) = resp.error {
            return Err(CapabilityError::InvocationFailed(err.message));
        }
        resp.result
            .ok_or_else(|| CapabilityError::InvocationFailed("empty result".to_string()))
    }
}

impl Drop for McpConnector {
    fn drop(&mut self) {
        self.disconnect();
    }
}
