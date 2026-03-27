use crate::cli_error::CliError;

const R: &str = "\x1b[0m";
const B: &str = "\x1b[1m";
const D: &str = "\x1b[2m";
const GR: &str = "\x1b[32m";
const YL: &str = "\x1b[33m";
const BL: &str = "\x1b[34m";
const CY: &str = "\x1b[36m";
const RD: &str = "\x1b[31m";

pub async fn handle(api_url: &str) -> Result<(), CliError> {
    let health = fetch_json(&format!("{api_url}/api/health")).await;
    let plans = fetch_json(&format!("{api_url}/api/plan-db/list")).await;
    let project = fetch_json(&format!("{api_url}/api/project/convergio/tree")).await;
    let proj_total = project.get("total_tasks").and_then(|v| v.as_i64()).unwrap_or(0);
    let proj_done = project.get("tasks_done").and_then(|v| v.as_i64()).unwrap_or(0);
    let proj_plans = project.get("plans").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);

    let version = health.get("version").and_then(|v| v.as_str()).unwrap_or("?");
    let peers = health.get("peers").and_then(|v| v.as_i64()).unwrap_or(0);
    let ok = health.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    let uptime = health.get("uptime_secs").and_then(|v| v.as_i64()).unwrap_or(0);

    let dot = if ok { format!("{GR}●{R}") } else { format!("{RD}●{R}") };

    // Get terminal width, default to 80
    let cols = term_width();

    println!();
    println!("{B}{CY}◆ Convergio{R} {D}v{version}{R} {dot} {D}{}{R} {BL}{peers}{R} peers", fmt_up(uptime));
    println!("{D}{}{R}", "─".repeat(cols.min(60)));

    let plan_list = plans.get("plans").and_then(|v| v.as_array());
    let Some(all) = plan_list else {
        println!("{RD}Cannot reach daemon{R}");
        return Ok(());
    };

    let mut active: Vec<&serde_json::Value> = Vec::new();
    let mut queued: Vec<&serde_json::Value> = Vec::new();

    for p in all {
        let st = p.get("status").and_then(|v| v.as_str()).unwrap_or("");
        match st {
            "doing" => active.push(p),
            "draft" | "todo" | "approved" => queued.push(p),
            _ => {}
        }
    }

    if !active.is_empty() {
        println!("{B}{YL}▶ In Progress{R}");
        for p in &active { print_plan(p, YL, "◉", cols); }
    } else {
        println!("{GR}✓ No active plans{R}");
    }

    if !queued.is_empty() {
        println!("{B}{BL}◇ Queued{R}");
        for p in &queued { print_plan(p, D, "○", cols); }
    }

    // Summary
    let t_done = if proj_done > 0 { proj_done } else { 0 };
    let t_total = if proj_total > 0 { proj_total } else { 0 };
    let pct = if t_total > 0 { t_done * 100 / t_total } else { 0 };
    let bar = progress_bar(pct as usize, 16);
    println!("{D}{}{R}", "─".repeat(cols.min(60)));
    println!("{B}{t_done}{R}/{t_total} tasks {bar} {B}{pct}%{R} {D}({proj_plans} plans){R}");
    println!();

    Ok(())
}

fn print_plan(p: &serde_json::Value, color: &str, icon: &str, cols: usize) {
    let id = p.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
    let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("?");
    let done = p.get("tasks_done").and_then(|v| v.as_i64()).unwrap_or(0);
    let total = p.get("tasks_total").and_then(|v| v.as_i64()).unwrap_or(0);
    // Truncate name to fit: icon(2) + # + id(4) + space + name + space + [d/t](8) < cols
    let max_name = cols.saturating_sub(22).min(35);
    let short: String = name.chars().take(max_name).collect();
    println!("{color}{icon}{R} #{id:<4} {short} {D}[{done}/{total}]{R}");
}

fn progress_bar(pct: usize, w: usize) -> String {
    let filled = (pct * w / 100).min(w);
    let empty = w - filled;
    format!("\x1b[42m{}\x1b[0m\x1b[48;5;236m{}\x1b[0m",
        " ".repeat(filled), " ".repeat(empty))
}

fn fmt_up(secs: i64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 { format!("{h}h{m}m") } else { format!("{m}m") }
}

fn term_width() -> usize {
    // Try COLUMNS env, fallback to 80
    std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(80)
}

async fn fetch_json(url: &str) -> serde_json::Value {
    match reqwest::get(url).await {
        Ok(resp) => resp.json().await.unwrap_or(serde_json::json!({})),
        Err(_) => serde_json::json!({}),
    }
}
