use crate::cli_error::CliError;

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

    let status = if ok { "online" } else { "OFFLINE" };

    println!();
    println!("Convergio v{version} [{status}] {peers} peers, up {}", fmt_up(uptime));
    println!("{}", "-".repeat(50));

    let plan_list = plans.get("plans").and_then(|v| v.as_array());
    let Some(all) = plan_list else {
        println!("ERROR: Cannot reach daemon");
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
        println!("IN PROGRESS:");
        for p in &active { print_plan(p, ">"); }
    } else {
        println!("No active plans.");
    }
    if !queued.is_empty() {
        println!("QUEUED:");
        for p in &queued { print_plan(p, " "); }
    }

    let pct = if proj_total > 0 { proj_done * 100 / proj_total } else { 0 };
    println!("{}", "-".repeat(50));
    println!("{}/{} tasks ({}%) | {} plans", proj_done, proj_total, pct, proj_plans);
    println!();
    Ok(())
}

fn print_plan(p: &serde_json::Value, marker: &str) {
    let id = p.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
    let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("?");
    let done = p.get("tasks_done").and_then(|v| v.as_i64()).unwrap_or(0);
    let total = p.get("tasks_total").and_then(|v| v.as_i64()).unwrap_or(0);
    let short: String = name.chars().take(35).collect();
    println!("  {marker} #{id} {short} [{done}/{total}]");
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
