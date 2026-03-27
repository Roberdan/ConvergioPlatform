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

    println!();
    println!("{B}{CY}  ◆ Convergio{R} {D}v{version}{R}  {dot} {D}{}{R}  {BL}{peers}{R} peers", fmt_up(uptime));
    println!("{D}  ────────────────────────────────────────────{R}");

    let plan_list = plans.get("plans").and_then(|v| v.as_array());
    let Some(all) = plan_list else {
        println!("  {RD}Cannot reach daemon{R}");
        return Ok(());
    };

    let mut active: Vec<&serde_json::Value> = Vec::new();
    let mut queued: Vec<&serde_json::Value> = Vec::new();
    let mut done_count: usize = 0;
    let mut done_tasks: i64 = 0;
    let mut total_tasks: i64 = 0;

    for p in all {
        let st = p.get("status").and_then(|v| v.as_str()).unwrap_or("");
        let td = p.get("tasks_done").and_then(|v| v.as_i64()).unwrap_or(0);
        let tt = p.get("tasks_total").and_then(|v| v.as_i64()).unwrap_or(0);
        match st {
            "doing" => { active.push(p); total_tasks += tt; done_tasks += td; }
            "done" | "completed" => { done_count += 1; total_tasks += tt; done_tasks += td; }
            "draft" | "todo" | "approved" => { queued.push(p); total_tasks += tt; }
            _ => {}
        }
    }

    if !active.is_empty() {
        println!("  {B}{YL}▶ In Progress{R}");
        for p in &active { print_plan(p, YL, "◉"); }
        println!();
    } else {
        println!("  {GR}No active plans{R}");
        println!();
    }

    if !queued.is_empty() {
        println!("  {B}{BL}◇ Queued{R}");
        for p in &queued { print_plan(p, D, "○"); }
        println!();
    }

    // Summary — use project tree for total counts (includes completed)
    let t_done = if proj_done > 0 { proj_done } else { done_tasks };
    let t_total = if proj_total > 0 { proj_total } else { total_tasks };
    let pct = if t_total > 0 { t_done * 100 / t_total } else { 0 };
    let bar = progress_bar(pct as usize, 20);
    println!("{D}  ────────────────────────────────────────────{R}");
    println!("  {B}{t_done}{R}/{t_total} tasks  {bar}  {B}{pct}%{R}  {D}({proj_plans} plans){R}");
    println!();

    Ok(())
}

fn print_plan(p: &serde_json::Value, color: &str, icon: &str) {
    let id = p.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
    let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("?");
    let done = p.get("tasks_done").and_then(|v| v.as_i64()).unwrap_or(0);
    let total = p.get("tasks_total").and_then(|v| v.as_i64()).unwrap_or(0);
    // Truncate name to 40 chars
    let short: String = name.chars().take(40).collect();
    println!("  {color}{icon}{R}  #{id:<4} {short:<40} {D}[{done}/{total}]{R}");
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

async fn fetch_json(url: &str) -> serde_json::Value {
    match reqwest::get(url).await {
        Ok(resp) => resp.json().await.unwrap_or(serde_json::json!({})),
        Err(_) => serde_json::json!({}),
    }
}
