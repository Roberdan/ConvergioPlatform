// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Unit tests for convergio-mcp-server: ring enforcement, tool registry, protocol parsing.
// Follows TDD: tests written before implementation to define expected contracts.

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
    use crate::mcp_server::security::{check_ring_access, McpError};
    use crate::mcp_server::tools::list_tools;
    use crate::capabilities::ring::Ring;

    // ── Ring enforcement ──────────────────────────────────────────────────────

    #[test]
    fn ring_core_can_access_all() {
        assert!(check_ring_access(Ring::Core, Ring::Core).is_ok());
        assert!(check_ring_access(Ring::Core, Ring::Trusted).is_ok());
        assert!(check_ring_access(Ring::Core, Ring::Community).is_ok());
        assert!(check_ring_access(Ring::Core, Ring::Sandboxed).is_ok());
    }

    #[test]
    fn ring_trusted_cannot_access_core() {
        assert!(check_ring_access(Ring::Trusted, Ring::Core).is_err());
        assert!(check_ring_access(Ring::Trusted, Ring::Trusted).is_ok());
        assert!(check_ring_access(Ring::Trusted, Ring::Community).is_ok());
    }

    #[test]
    fn ring_community_cannot_access_trusted_or_core() {
        assert!(check_ring_access(Ring::Community, Ring::Core).is_err());
        assert!(check_ring_access(Ring::Community, Ring::Trusted).is_err());
        assert!(check_ring_access(Ring::Community, Ring::Community).is_ok());
        assert!(check_ring_access(Ring::Community, Ring::Sandboxed).is_ok());
    }

    #[test]
    fn ring_sandboxed_cannot_access_community_or_above() {
        assert!(check_ring_access(Ring::Sandboxed, Ring::Core).is_err());
        assert!(check_ring_access(Ring::Sandboxed, Ring::Trusted).is_err());
        assert!(check_ring_access(Ring::Sandboxed, Ring::Community).is_err());
        assert!(check_ring_access(Ring::Sandboxed, Ring::Sandboxed).is_ok());
    }

    #[test]
    fn ring_violation_error_contains_ring_levels() {
        let err = check_ring_access(Ring::Community, Ring::Trusted).unwrap_err();
        match err {
            McpError::RingViolation { caller, required } => {
                assert_eq!(caller, 2);
                assert_eq!(required, 1);
            }
            _ => panic!("expected RingViolation, got {:?}", err),
        }
    }

    // ── Tool registry ─────────────────────────────────────────────────────────

    #[test]
    fn list_tools_ring0_returns_14_tools() {
        let tools = list_tools(Ring::Core);
        assert_eq!(tools.len(), 14, "Ring 0 must expose all 14 tools, got {}", tools.len());
    }

    #[test]
    fn list_tools_ring1_returns_13_tools() {
        // Ring 1 (Trusted) sees all tools except cvg_restart_node (Ring 0 only).
        let tools = list_tools(Ring::Trusted);
        assert_eq!(tools.len(), 13, "Ring 1 must expose 13 tools, got {}", tools.len());
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(!names.contains(&"cvg_restart_node"), "restart_node is ring-0 only");
    }

    #[test]
    fn list_tools_ring2_returns_read_only_subset() {
        let tools = list_tools(Ring::Community);
        // Community: list_plans, get_plan, list_agents, mesh_status, node_readiness,
        //            cost_summary, kernel_status — no write/action tools
        assert!(tools.len() >= 7, "Ring 2 must have at least 7 read tools");
        // Write/action tools must NOT appear for community ring
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(!names.contains(&"cvg_update_task"), "update_task is ring-1 only");
        assert!(!names.contains(&"cvg_checkpoint_save"), "checkpoint_save is ring-1 only");
        assert!(!names.contains(&"cvg_agent_start"), "agent_start is ring-1 only");
        assert!(!names.contains(&"cvg_restart_node"), "restart_node is ring-0 only");
    }

    #[test]
    fn list_tools_ring3_returns_sandboxed_subset() {
        let tools = list_tools(Ring::Sandboxed);
        // Sandboxed: only read plans and agents
        assert!(tools.len() >= 2, "Ring 3 must have at least 2 tools");
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"cvg_list_plans"), "list_plans must be available for sandboxed");
        assert!(names.contains(&"cvg_list_agents"), "list_agents must be available for sandboxed");
        assert!(!names.contains(&"cvg_notify"), "notify is ring-1 only");
    }

    #[test]
    fn all_tools_have_valid_input_schema() {
        let tools = list_tools(Ring::Core);
        for tool in &tools {
            assert!(!tool.name.is_empty(), "tool name must not be empty");
            assert!(!tool.description.is_empty(), "tool description must not be empty");
            // input_schema must be a JSON object with "type" field
            assert_eq!(
                tool.input_schema.get("type").and_then(|v| v.as_str()),
                Some("object"),
                "tool {} input_schema must have type:object",
                tool.name
            );
        }
    }

    #[test]
    fn tool_names_use_cvg_prefix() {
        let tools = list_tools(Ring::Core);
        for tool in &tools {
            assert!(
                tool.name.starts_with("cvg_"),
                "tool name '{}' must start with cvg_",
                tool.name
            );
        }
    }

    // ── Protocol parsing ──────────────────────────────────────────────────────

    #[test]
    fn parse_valid_initialize_request() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}"#;
        let req: JsonRpcRequest = serde_json::from_str(raw).expect("should parse initialize request");
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, Some(json!(1)));
        assert_eq!(req.method, "initialize");
    }

    #[test]
    fn parse_tools_list_request() {
        let raw = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;
        let req: JsonRpcRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.method, "tools/list");
        assert_eq!(req.id, Some(json!(2)));
    }

    #[test]
    fn parse_tools_call_request() {
        let raw = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"cvg_list_plans","arguments":{}}}"#;
        let req: JsonRpcRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.method, "tools/call");
        let params = req.params.unwrap();
        assert_eq!(params.get("name").and_then(|v| v.as_str()), Some("cvg_list_plans"));
    }

    #[test]
    fn parse_malformed_json_returns_error() {
        let raw = "not valid json{{{";
        let result = serde_json::from_str::<JsonRpcRequest>(raw);
        assert!(result.is_err(), "malformed JSON must fail to parse");
    }

    #[test]
    fn response_result_serializes_correctly() {
        let resp = JsonRpcResponse::result(json!(1), json!({"protocolVersion": "2024-11-05"}));
        let s = serde_json::to_string(&resp).unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v.get("jsonrpc").and_then(|v| v.as_str()), Some("2.0"));
        assert!(v.get("result").is_some());
        assert!(v.get("error").is_none());
    }

    #[test]
    fn response_error_serializes_correctly() {
        let resp = JsonRpcResponse::error(json!(1), -32601, "Method not found");
        let s = serde_json::to_string(&resp).unwrap();
        let v: Value = serde_json::from_str(&s).unwrap();
        assert!(v.get("error").is_some());
        assert!(v.get("result").is_none());
        let err = v.get("error").unwrap();
        assert_eq!(err.get("code").and_then(|v| v.as_i64()), Some(-32601));
    }

    // ── Integration: McpServer handle_request ────────────────────────────────

    #[test]
    fn handle_initialize_returns_server_info() {
        use crate::mcp_server::McpServer;
        let server = McpServer::new(1, "http://localhost:1", None);
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}"#;
        let resp_str = server.handle_request(raw);
        let v: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(v.get("jsonrpc").and_then(|v| v.as_str()), Some("2.0"));
        let result = v.get("result").expect("initialize must return result");
        assert_eq!(
            result.get("serverInfo").and_then(|s| s.get("name")).and_then(|v| v.as_str()),
            Some("convergio-mcp-server")
        );
    }

    #[test]
    fn handle_tools_list_returns_tools_array() {
        use crate::mcp_server::McpServer;
        let server = McpServer::new(1, "http://localhost:1", None);
        let raw = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;
        let resp_str = server.handle_request(raw);
        let v: Value = serde_json::from_str(&resp_str).unwrap();
        let tools = v.get("result").and_then(|r| r.get("tools")).expect("tools/list must return tools");
        assert!(tools.is_array(), "tools must be an array");
        let arr = tools.as_array().unwrap();
        assert_eq!(arr.len(), 13, "ring 1 must see 13 tools (restart_node is ring-0 only)");
    }

    #[test]
    fn handle_unknown_method_returns_method_not_found() {
        use crate::mcp_server::McpServer;
        let server = McpServer::new(1, "http://localhost:1", None);
        let raw = r#"{"jsonrpc":"2.0","id":5,"method":"unknown/method","params":{}}"#;
        let resp_str = server.handle_request(raw);
        let v: Value = serde_json::from_str(&resp_str).unwrap();
        let err = v.get("error").expect("unknown method must return error");
        assert_eq!(err.get("code").and_then(|c| c.as_i64()), Some(-32601));
    }

    #[test]
    fn handle_tools_call_ring_violation_returns_ring_error() {
        use crate::mcp_server::McpServer;
        // Ring 2 (Community) tries to call cvg_update_task (Ring 1 required)
        let server = McpServer::new(2, "http://localhost:1", None);
        let raw = r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"cvg_update_task","arguments":{"task_id":1,"status":"done"}}}"#;
        let resp_str = server.handle_request(raw);
        let v: Value = serde_json::from_str(&resp_str).unwrap();
        let err = v.get("error").expect("ring violation must return error");
        assert_eq!(err.get("code").and_then(|c| c.as_i64()), Some(-32001));
    }

    #[test]
    fn handle_tools_call_daemon_unreachable_returns_daemon_error() {
        use crate::mcp_server::McpServer;
        // Daemon at port 1 is unreachable
        let server = McpServer::new(1, "http://localhost:1", None);
        let raw = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"cvg_list_plans","arguments":{}}}"#;
        let resp_str = server.handle_request(raw);
        let v: Value = serde_json::from_str(&resp_str).unwrap();
        // Must return error, not crash. Code -32002 (daemon unreachable) or -32003 (daemon error).
        assert!(v.get("error").is_some(), "unreachable daemon must return error, not panic");
    }
}
