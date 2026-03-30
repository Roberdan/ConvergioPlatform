use super::*;

#[test]
fn tool_definitions_returns_seven_tools() {
    let defs = tool_definitions();
    assert_eq!(defs.len(), 7);
    let names: Vec<&str> = defs.iter().map(|d| d.name).collect();
    assert!(names.contains(&"get_plans"));
    assert!(names.contains(&"get_plan_detail"));
    assert!(names.contains(&"get_costs"));
    assert!(names.contains(&"get_node_status"));
    assert!(names.contains(&"get_kernel_status"));
    assert!(names.contains(&"get_agents"));
    assert!(names.contains(&"restart_node"));
}

#[test]
fn call_tool_unknown_returns_none() {
    let result = call_tool("nonexistent", "http://localhost:1", &serde_json::json!({}));
    assert!(result.is_none());
}

#[test]
fn call_tool_get_plan_detail_missing_arg_returns_none() {
    // plan_id not provided — should return None
    let result = call_tool("get_plan_detail", "http://localhost:1", &serde_json::json!({}));
    assert!(result.is_none());
}

#[test]
fn call_tool_dispatches_get_plans() {
    // Daemon unreachable → returns error JSON (not None)
    let result = call_tool("get_plans", "http://localhost:1", &serde_json::json!({}));
    assert!(result.is_some());
    let v: Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert!(v.get("error").is_some() || v.is_array());
}

#[test]
fn call_tool_restart_node_uses_target_arg() {
    let result = call_tool(
        "restart_node",
        "http://localhost:1",
        &serde_json::json!({"target": "macProM1"}),
    );
    assert!(result.is_some());
    // Unreachable → error JSON
    let v: Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert!(v.get("error").is_some() || v.get("status").is_some());
}
