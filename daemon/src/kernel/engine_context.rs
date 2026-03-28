// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Context gathering and output-processing helpers for KernelEngine.

use crate::kernel::tools;

/// Smart context gathering: picks APIs based on question keywords.
/// The kernel (Rust, deterministic) does the retrieval; the LLM just reasons.
/// Public wrapper for voice_router Ali escalation.
pub fn smart_context_gather_pub(question: &str, daemon_url: &str) -> String {
    smart_context_gather(question, daemon_url)
}

pub(crate) fn smart_context_gather(question: &str, daemon_url: &str) -> String {
    let q = question.to_lowercase();
    let mut ctx = String::new();

    // Always include plan summary (fast, small)
    if let Some(plans) = tools::call_tool("get_plans", daemon_url, &serde_json::json!({})) {
        ctx += &format!("Piani:\n{plans}\n\n");
    }

    // Cost data if money-related
    if q.contains("cost") || q.contains("spes") || q.contains("soldi") || q.contains("dollari") || q.contains("budget") {
        if let Some(costs) = tools::call_tool("get_costs", daemon_url, &serde_json::json!({})) {
            ctx += &format!("Costi:\n{costs}\n\n");
        }
    }

    // Node/kernel if infra-related
    if q.contains("nod") || q.contains("kernel") || q.contains("m1") || q.contains("m5") || q.contains("mesh") || q.contains("macchina") {
        if let Some(node) = tools::call_tool("get_node_status", daemon_url, &serde_json::json!({})) {
            ctx += &format!("Nodo:\n{node}\n\n");
        }
        if let Some(kernel) = tools::call_tool("get_kernel_status", daemon_url, &serde_json::json!({})) {
            ctx += &format!("Kernel:\n{kernel}\n\n");
        }
    }

    // Agents if team-related
    if q.contains("agent") || q.contains("chi") || q.contains("team") || q.contains("ali") {
        if let Some(agents) = tools::call_tool("get_agents", daemon_url, &serde_json::json!({})) {
            ctx += &format!("Agenti:\n{agents}\n\n");
        }
    }

    // Plan detail if specific plan mentioned
    if q.contains("piano") || q.contains("plan") {
        // Try to extract a plan ID
        let id: Option<u32> = q.split_whitespace().filter_map(|w| w.parse().ok()).next();
        if let Some(plan_id) = id {
            let args = serde_json::json!({"plan_id": plan_id});
            if let Some(detail) = tools::call_tool("get_plan_detail", daemon_url, &args) {
                ctx += &format!("Dettaglio piano {plan_id}:\n{detail}\n\n");
            }
        }
    }

    // For generic questions, add costs + kernel status too
    if ctx.len() < 200 {
        if let Some(costs) = tools::call_tool("get_costs", daemon_url, &serde_json::json!({})) {
            ctx += &format!("Costi:\n{costs}\n\n");
        }
        if let Some(kernel) = tools::call_tool("get_kernel_status", daemon_url, &serde_json::json!({})) {
            ctx += &format!("Kernel:\n{kernel}\n\n");
        }
    }

    if ctx.is_empty() {
        "Nessun dato disponibile dal daemon.".to_string()
    } else {
        ctx
    }
}

/// Parse the first <tool_call>...</tool_call> block from model output.
/// Returns (tool_name, arguments_json_string) or None if no call found.
#[allow(dead_code)] // Used in tests; will be wired when multi-round tool calling lands.
pub(crate) fn extract_tool_call(text: &str) -> Option<(String, String)> {
    let start_tag = "<tool_call>";
    let end_tag = "</tool_call>";
    let start = text.find(start_tag)?;
    let end = text.find(end_tag)?;
    let json_str = text[start + start_tag.len()..end].trim();
    let v: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let name = v.get("name")?.as_str()?.to_string();
    let args = v
        .get("arguments")
        .map(|a| a.to_string())
        .unwrap_or_else(|| "{}".to_string());
    Some((name, args))
}

/// Strip mlx_lm debug output from model response.
/// Removes lines starting with "=", "Prompt:", "Generation:", "Peak memory:".
pub(crate) fn strip_mlx_debug(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let t = line.trim();
            !t.starts_with('=')
                && !t.starts_with("Prompt:")
                && !t.starts_with("Generation:")
                && !t.starts_with("Peak memory:")
                && !t.starts_with("Calling `python")
                && !t.starts_with("Use `mlx_lm")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_tool_call_parses_no_args() {
        let text = r#"Sure! <tool_call>{"name":"get_plans","arguments":{}}</tool_call>"#;
        let result = extract_tool_call(text);
        assert!(result.is_some());
        let (name, args) = result.unwrap();
        assert_eq!(name, "get_plans");
        assert_eq!(args, "{}");
    }

    #[test]
    fn extract_tool_call_parses_with_args() {
        let text =
            r#"<tool_call>{"name":"get_plan_detail","arguments":{"plan_id":729}}</tool_call>"#;
        let result = extract_tool_call(text);
        assert!(result.is_some());
        let (name, args) = result.unwrap();
        assert_eq!(name, "get_plan_detail");
        // arguments JSON must contain plan_id 729
        let v: serde_json::Value = serde_json::from_str(&args).unwrap();
        assert_eq!(v.get("plan_id").and_then(|v| v.as_u64()), Some(729));
    }

    #[test]
    fn extract_tool_call_returns_none_when_absent() {
        let text = "Ecco la risposta senza tool call.";
        assert!(extract_tool_call(text).is_none());
    }

    #[test]
    fn extract_tool_call_returns_none_on_malformed_json() {
        let text = "<tool_call>not json at all</tool_call>";
        assert!(extract_tool_call(text).is_none());
    }

    #[test]
    fn strip_mlx_debug_removes_debug_lines() {
        let raw = "=====\nPrompt: 10\nGeneration: 5\nPeak memory: 1.2 GB\nRisposta utile";
        let cleaned = strip_mlx_debug(raw);
        assert_eq!(cleaned, "Risposta utile");
    }

    #[test]
    fn strip_mlx_debug_keeps_normal_lines() {
        let raw = "Ci sono 3 piani attivi.\nIl piano 734 e' in corso.";
        let cleaned = strip_mlx_debug(raw);
        assert_eq!(cleaned, raw);
    }
}
