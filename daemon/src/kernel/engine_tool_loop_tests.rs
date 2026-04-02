// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for the multi-round tool calling loop.

use super::*;

#[test]
fn tool_descriptions_block_contains_all_tools() {
    let block = tool_descriptions_block();
    let catalog = crate::kernel::tools::ToolCatalog::all();
    // Verify at least the plan tools are present
    assert!(block.contains("get_plans"), "missing get_plans");
    assert!(block.contains("get_plan_detail"), "missing get_plan_detail");
    assert!(catalog.tools.len() >= 4, "catalog too small");
}

#[test]
fn tool_descriptions_block_shows_format() {
    let block = tool_descriptions_block();
    assert!(block.contains("<tool_call>"));
    assert!(block.contains("</tool_call>"));
}

#[test]
fn strip_tool_preamble_returns_text_after_tag() {
    let text = "Let me check. <tool_call>{\"name\":\"x\",\"arguments\":{}}</tool_call>\nHere is the answer.";
    let result = strip_tool_preamble(text);
    assert_eq!(result, "Here is the answer.");
}

#[test]
fn strip_tool_preamble_returns_full_text_when_no_tag() {
    let text = "Just a normal response without tools.";
    let result = strip_tool_preamble(text);
    assert_eq!(result, text);
}

#[test]
fn strip_tool_preamble_returns_full_when_nothing_after_tag() {
    let text = "<tool_call>{\"name\":\"x\",\"arguments\":{}}</tool_call>";
    let result = strip_tool_preamble(text);
    assert_eq!(result, text);
}

#[test]
fn max_tool_rounds_is_reasonable() {
    assert!(MAX_TOOL_ROUNDS >= 2, "need at least 2 rounds");
    assert!(MAX_TOOL_ROUNDS <= 5, "too many rounds risks timeout");
}
