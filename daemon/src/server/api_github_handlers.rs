// api_github_handlers: GitHub stats handler + gh CLI helper functions
use super::state::{query_rows, ApiError, ServerState};
use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

pub async fn handle_github_stats(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;

    // Get plan + project github_url via JOIN
    let plan_info = query_rows(
        &conn,
        "SELECT p.id, p.name, p.project_id, pr.github_url FROM plans p LEFT JOIN projects pr ON LOWER(p.project_id) = LOWER(pr.id) WHERE p.id=?1",
        rusqlite::params![plan_id],
    )?;
    if plan_info.is_empty() {
        return Err(ApiError::bad_request(format!("plan {plan_id} not found")));
    }
    let plan = &plan_info[0];
    let github_url = plan
        .get("github_url")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let repo_nwo = extract_nwo(github_url);

    // Local commit stats from DB
    let commit_totals = query_rows(
        &conn,
        "SELECT COUNT(*) AS commit_count, COALESCE(SUM(lines_added),0) AS lines_added, COALESCE(SUM(lines_removed),0) AS lines_removed, COALESCE(SUM(files_changed),0) AS files_changed FROM plan_commits WHERE plan_id=?1",
        rusqlite::params![plan_id],
    ).unwrap_or_default();

    let event_totals = query_rows(
        &conn,
        "SELECT status, COUNT(*) AS count FROM github_events WHERE plan_id=?1 GROUP BY status",
        rusqlite::params![plan_id],
    )
    .unwrap_or_default();

    // If we have a real GitHub repo, fetch live stats via gh CLI
    let (repo_stats, live_stats) = if !repo_nwo.is_empty() {
        (fetch_repo_stats(&repo_nwo), fetch_live_stats(&repo_nwo))
    } else {
        (json!({}), json!({}))
    };

    let ct = commit_totals.first().cloned().unwrap_or_else(|| json!({}));
    let lines_added = ct.get("lines_added").and_then(|v| v.as_i64()).unwrap_or(0);
    let lines_removed = ct
        .get("lines_removed")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "repo": if repo_nwo.is_empty() { "local/repo" } else { &repo_nwo },
        "github_issue": Value::Null,
        "commit_totals": ct,
        "lines_changed": lines_added + lines_removed + live_stats.get("lines_changed").and_then(|v| v.as_i64()).unwrap_or(0),
        "commits_today": live_stats.get("commits_today").and_then(|v| v.as_i64()).unwrap_or(0),
        "open_prs": live_stats.get("open_prs").and_then(|v| v.as_i64()).unwrap_or(0),
        "pr_merge_velocity": live_stats.get("pr_merge_velocity").and_then(|v| v.as_f64()).unwrap_or(0.0),
        "event_totals": event_totals,
        "repo_stats": repo_stats,
    })))
}

/// Extract "owner/repo" from a GitHub URL
pub fn extract_nwo(url: &str) -> String {
    // Handles: https://github.com/Owner/Repo, https://github.com/Owner/Repo.git
    let stripped = url.trim_end_matches(".git").trim_end_matches('/');
    if let Some(rest) = stripped.strip_prefix("https://github.com/") {
        let parts: Vec<&str> = rest.splitn(3, '/').collect();
        if parts.len() >= 2 {
            return format!("{}/{}", parts[0], parts[1]);
        }
    }
    String::new()
}

/// Fetch repo metadata via `gh repo view`
pub fn fetch_repo_stats(nwo: &str) -> Value {
    let Ok(out) = std::process::Command::new("gh")
        .args([
            "repo",
            "view",
            nwo,
            "--json",
            "nameWithOwner,stargazerCount,forkCount,openIssues",
        ])
        .output()
    else {
        return json!({});
    };
    if !out.status.success() {
        return json!({});
    }
    serde_json::from_slice(&out.stdout).unwrap_or(json!({}))
}

/// Fetch live commit/PR stats via `gh api`
pub fn fetch_live_stats(nwo: &str) -> Value {
    // Open PRs
    let open_prs = std::process::Command::new("gh")
        .args([
            "pr", "list", "--repo", nwo, "--state", "open", "--json", "number", "--limit", "100",
        ])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                serde_json::from_slice::<Value>(&o.stdout).ok()
            } else {
                None
            }
        })
        .and_then(|v| v.as_array().map(|a| a.len() as i64))
        .unwrap_or(0);

    // Today's date as ISO string
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let days_since_epoch = secs / 86400;
    // Simple date calc (good enough for UTC today)
    let today_epoch_start = days_since_epoch * 86400;
    let today = format_epoch_date(today_epoch_start);
    let week_ago = format_epoch_date(today_epoch_start - 7 * 86400);

    // Commits today on default branch
    let commits_today = std::process::Command::new("gh")
        .args([
            "api",
            &format!("repos/{nwo}/commits?since={today}T00:00:00Z&per_page=100"),
            "--jq",
            "length",
        ])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(0);

    // Merged PRs in last 7 days for velocity
    let merged_week = std::process::Command::new("gh")
        .args([
            "pr", "list", "--repo", nwo, "--state", "merged", "--json", "mergedAt", "--limit",
            "100",
        ])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                serde_json::from_slice::<Value>(&o.stdout).ok()
            } else {
                None
            }
        })
        .and_then(|v| {
            v.as_array().map(|a| {
                a.iter()
                    .filter(|pr| {
                        pr.get("mergedAt")
                            .and_then(|d| d.as_str())
                            .map(|d| d >= format!("{week_ago}T00:00:00Z").as_str())
                            .unwrap_or(false)
                    })
                    .count() as f64
            })
        })
        .unwrap_or(0.0);
    let velocity = merged_week / 7.0;

    // Lines changed this week (from GitHub code_frequency stats — weekly granularity)
    let lines_changed: i64 = std::process::Command::new("gh")
        .args(["api", &format!("repos/{nwo}/stats/code_frequency")])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                serde_json::from_slice::<Value>(&o.stdout).ok()
            } else {
                None
            }
        })
        .and_then(|v| v.as_array().and_then(|a| a.last().cloned()))
        .map(|week| {
            let add = week.get(1).and_then(|v| v.as_i64()).unwrap_or(0);
            let del = week.get(2).and_then(|v| v.as_i64()).unwrap_or(0);
            add + del.abs()
        })
        .unwrap_or(0);

    json!({
        "open_prs": open_prs,
        "commits_today": commits_today,
        "pr_merge_velocity": velocity,
        "lines_changed": lines_changed,
    })
}

/// Format epoch seconds (midnight UTC) as YYYY-MM-DD
pub fn format_epoch_date(epoch_secs: u64) -> String {
    // Civil date from epoch days
    let days = (epoch_secs / 86400) as i64;
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}
