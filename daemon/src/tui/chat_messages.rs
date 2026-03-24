// Chat message builders and context enrichment.
// Extracted from chat_handler.rs — pure data manipulation, no I/O.

use crate::tui::data::{ChatMessage, TuiData};

/// Enrich user message with live platform data so Ali can answer without tools.
pub fn enrich_with_context(question: &str, data: &TuiData) -> String {
    let mut ctx = String::from("[Convergio Platform — live snapshot]\n");

    // Plans
    if !data.plans.is_empty() {
        ctx.push_str("Plans:\n");
        for p in &data.plans {
            ctx.push_str(&format!(
                "  #{} {} | {} | {}/{} tasks\n",
                p.id, p.name, p.status, p.tasks_done, p.tasks_total
            ));
        }
    }

    // KPIs
    ctx.push_str(&format!(
        "KPI: {} active plans, {} agents, {}k tokens today, ${:.2} cost, {} mesh peers\n",
        data.kpis.plans_active, data.kpis.agents_running,
        data.kpis.daily_tokens / 1000, data.kpis.daily_cost,
        data.kpis.mesh_online,
    ));

    // Agents
    if !data.agents.is_empty() {
        ctx.push_str("Agents:\n");
        for a in &data.agents {
            let task = a.active_task.as_deref().unwrap_or("-");
            ctx.push_str(&format!("  {} ({}) on {} | {}\n", a.name, a.role, a.host, task));
        }
    }

    // Mesh
    if !data.mesh_nodes.is_empty() {
        ctx.push_str("Mesh:\n");
        for n in &data.mesh_nodes {
            let status = if n.online { "online" } else { "offline" };
            ctx.push_str(&format!(
                "  {} | {} | {} | cpu:{:.0}%\n",
                n.name, n.role, status, n.cpu_percent
            ));
        }
    }

    // Pipeline (active tasks)
    if !data.pipeline.is_empty() {
        ctx.push_str("Active tasks:\n");
        for t in data.pipeline.iter().take(10) {
            ctx.push_str(&format!(
                "  {} {} | {} | agent:{}\n",
                t.task_id, t.title, t.status, t.agent
            ));
        }
    }

    ctx.push_str("[End snapshot]\n\n");
    ctx.push_str("Answer from this data WITHOUT using tools when possible. ");
    ctx.push_str("Only use tools for actions (create plan, stop agent, etc.).\n\n");
    ctx.push_str(question);
    ctx
}

/// Build a `ChatMessage` for the user turn and append it to `data`.
pub fn push_user_message(data: &mut TuiData, content: &str) {
    data.chat_messages.push(ChatMessage {
        role: "user".to_string(),
        content: content.to_string(),
        timestamp: chrono_now(),
    });
}

/// Build a `ChatMessage` for the assistant turn and append it to `data`.
pub fn push_assistant_message(data: &mut TuiData, content: &str) {
    data.chat_messages.push(ChatMessage {
        role: "assistant".to_string(),
        content: content.to_string(),
        timestamp: chrono_now(),
    });
}

/// Returns the current timestamp as ISO-8601 string (UTC).
pub(crate) fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let s = secs;
    let sec = s % 60;
    let min = (s / 60) % 60;
    let hour = (s / 3600) % 24;
    let days = s / 86400;
    let year = 1970 + days / 365;
    let day_of_year = days % 365;
    let month = day_of_year / 30 + 1;
    let day = day_of_year % 30 + 1;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::data::TuiData;

    #[test]
    fn push_user_message_appends_with_user_role() {
        let mut data = TuiData::default();
        push_user_message(&mut data, "hello world");
        assert_eq!(data.chat_messages.len(), 1);
        assert_eq!(data.chat_messages[0].role, "user");
        assert_eq!(data.chat_messages[0].content, "hello world");
    }

    #[test]
    fn push_assistant_message_appends_with_assistant_role() {
        let mut data = TuiData::default();
        push_assistant_message(&mut data, "I am Convergio");
        assert_eq!(data.chat_messages.len(), 1);
        assert_eq!(data.chat_messages[0].role, "assistant");
        assert_eq!(data.chat_messages[0].content, "I am Convergio");
    }

    #[test]
    fn chrono_now_produces_nonempty_string() {
        let ts = chrono_now();
        assert!(!ts.is_empty());
        assert!(ts.contains('T'), "expected ISO-like timestamp");
    }
}
