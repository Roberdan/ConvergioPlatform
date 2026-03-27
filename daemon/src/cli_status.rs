use crate::cli_error::CliError;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const CYAN: &str = "\x1b[36m";
const WHITE: &str = "\x1b[37m";
const BG_GREEN: &str = "\x1b[42m";
const BG_DARK: &str = "\x1b[48;5;236m";

pub async fn handle(api_url: &str) -> Result<(), CliError> {
    let health = fetch_json(&format!("{api_url}/api/health")).await;
    let plans = fetch_json(&format!("{api_url}/api/plan-db/list")).await;

    let version = health.get("version").and_then(|v| v.as_str()).unwrap_or("?");
    let peers = health.get("peers").and_then(|v| v.as_i64()).unwrap_or(0);
    let ok = health.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    let uptime = health.get("uptime_secs").and_then(|v| v.as_i64()).unwrap_or(0);

    // Header
    println!();
    println!("  {BOLD}{CYAN}◆ Convergio Platform{RESET}  {DIM}v{version}{RESET}");
    println!("  {DIM}─────────────────────────────────────────{RESET}");

    // Connection status
    let status_icon = if ok { format!("{GREEN}●{RESET}") } else { format!("\x1b[31m●{RESET}") };
    println!("  {status_icon} Daemon {DIM}localhost:8420{RESET}  │  {BLUE}{peers}{RESET} peers  │  uptime {DIM}{}{RESET}", format_duration(uptime));
    println!();

    // Plans
    let plan_list = plans.get("plans").and_then(|v| v.as_array());
    if let Some(plans) = plan_list {
        let mut active: Vec<&serde_json::Value> = Vec::new();
        let mut recent: Vec<&serde_json::Value> = Vec::new();
        let mut queued: Vec<&serde_json::Value> = Vec::new();

        for p in plans {
            let st = p.get("status").and_then(|v| v.as_str()).unwrap_or("");
            match st {
                "doing" => active.push(p),
                "done" | "completed" => recent.push(p),
                "draft" | "todo" | "approved" => queued.push(p),
                _ => {}
            }
        }

        // Active plans
        if !active.is_empty() {
            println!("  {BOLD}{YELLOW}▶ Active Plans{RESET}");
            for p in &active {
                print_plan(p, "doing");
            }
            println!();
        }

        // Queued plans
        if !queued.is_empty() {
            println!("  {BOLD}{BLUE}◇ Queued{RESET}");
            for p in &queued {
                let st = p.get("status").and_then(|v| v.as_str()).unwrap_or("draft");
                print_plan(p, st);
            }
            println!();
        }

        // Recently completed (last 10)
        let recent_slice = if recent.len() > 10 { &recent[recent.len()-10..] } else { &recent };
        if !recent_slice.is_empty() {
            println!("  {BOLD}{GREEN}✓ Recently Completed{RESET}  {DIM}({} total){RESET}", recent.len());
            for p in recent_slice {
                print_plan(p, "done");
            }
            println!();
        }

        // Summary bar
        let total_done: i64 = plans.iter()
            .filter(|p| matches!(p.get("status").and_then(|v| v.as_str()), Some("done" | "completed")))
            .filter_map(|p| p.get("tasks_done").and_then(|v| v.as_i64()))
            .sum();
        let total_tasks: i64 = plans.iter()
            .filter(|p| p.get("status").and_then(|v| v.as_str()) != Some("cancelled"))
            .filter_map(|p| p.get("tasks_total").and_then(|v| v.as_i64()))
            .sum();
        let pct = if total_tasks > 0 { (total_done as f64 / total_tasks as f64 * 100.0) as i64 } else { 0 };

        println!("  {DIM}─────────────────────────────────────────{RESET}");
        print!("  {BOLD}{total_done}/{total_tasks}{RESET} tasks  ");
        print_progress_bar(pct, 20);
        println!("  {BOLD}{pct}%{RESET}");
    }

    println!();
    Ok(())
}

fn print_plan(p: &serde_json::Value, status: &str) {
    let id = p.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
    let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("?");
    let done = p.get("tasks_done").and_then(|v| v.as_i64()).unwrap_or(0);
    let total = p.get("tasks_total").and_then(|v| v.as_i64()).unwrap_or(0);

    let (icon, color) = match status {
        "doing" => ("◉", YELLOW),
        "done" | "completed" => ("✓", GREEN),
        "draft" => ("○", DIM),
        _ => ("◇", BLUE),
    };

    let pct = if total > 0 { format!(" {}%", done * 100 / total) } else { String::new() };
    println!("    {color}{icon}{RESET} {DIM}#{id:<4}{RESET} {WHITE}{name}{RESET}  {DIM}[{done}/{total}]{pct}{RESET}");
}

fn print_progress_bar(pct: i64, width: usize) {
    let filled = (pct as usize * width / 100).min(width);
    let empty = width - filled;
    print!("{BG_GREEN}{BOLD}");
    for _ in 0..filled { print!(" "); }
    print!("{RESET}{BG_DARK}");
    for _ in 0..empty { print!(" "); }
    print!("{RESET}");
}

fn format_duration(secs: i64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 { format!("{h}h {m}m") } else { format!("{m}m") }
}

async fn fetch_json(url: &str) -> serde_json::Value {
    match reqwest::get(url).await {
        Ok(resp) => resp.json().await.unwrap_or(serde_json::json!({})),
        Err(_) => serde_json::json!({}),
    }
}
