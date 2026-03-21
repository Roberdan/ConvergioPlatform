// overview: /api/overview handler — KPIs, lines-changed cache, agent counts
use super::super::state::{query_one, ApiError, ServerState};
use axum::extract::State;
use axum::Json;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::env;
use std::process::Command;
use std::sync::Mutex;
use std::time::Instant;

struct LinesCache {
    today: i64,
    week: i64,
    yesterday: i64,
    prev_week: i64,
    fetched_at: Instant,
}

static LINES_CACHE: Mutex<Option<LinesCache>> = Mutex::new(None);
const LINES_CACHE_TTL_SECS: u64 = 120;

pub async fn api_overview(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let row = query_one(
        &conn,
        "WITH plan_stats AS (
           SELECT COUNT(*) AS total,
                  SUM(CASE WHEN status IN ('todo','doing') THEN 1 ELSE 0 END) AS active,
                  SUM(CASE WHEN status='done' THEN 1 ELSE 0 END) AS done
           FROM plans
         ), task_stats AS (
           SELECT SUM(CASE WHEN status='in_progress' THEN 1 ELSE 0 END) AS tasks_running,
                  SUM(CASE WHEN status='blocked' THEN 1 ELSE 0 END) AS blocked
           FROM tasks WHERE status IN ('in_progress','blocked')
         ), agent_stats AS (
           SELECT COUNT(*) AS agents_running
           FROM agent_activity WHERE status='running'
         ), token_stats AS (
           SELECT COALESCE(SUM(input_tokens + output_tokens),0) AS total_tokens,
                  COALESCE(SUM(cost_usd),0) AS total_cost,
                  COALESCE(SUM(CASE WHEN date(created_at)=date('now') THEN input_tokens + output_tokens ELSE 0 END),0) AS today_tokens,
                  COALESCE(SUM(CASE WHEN date(created_at)=date('now') THEN cost_usd ELSE 0 END),0) AS today_cost
           FROM token_usage
         ), mesh_stats AS (
           SELECT COUNT(*) AS mesh_total,
                  SUM(CASE WHEN (strftime('%s','now') - COALESCE(last_seen,0)) < 300 THEN 1 ELSE 0 END) AS mesh_online
           FROM peer_heartbeats
         )
         SELECT p.total AS plans_total, p.active AS plans_active, p.done AS plans_done,
                COALESCE(a.agents_running,0) AS agents_running, COALESCE(t.blocked,0) AS blocked,
                tk.total_tokens, tk.total_cost, tk.today_tokens, tk.today_cost,
                m.mesh_total, m.mesh_online
         FROM plan_stats p, task_stats t, agent_stats a, token_stats tk, mesh_stats m",
        [],
    )?
    .unwrap_or_else(|| json!({}));

    // Return cached lines instantly; spawn background refresh if stale
    let (today_lines, week_lines, yesterday_lines, prev_week_lines) = {
        let cached = LINES_CACHE.lock().unwrap();
        if let Some(ref c) = *cached {
            let vals = (c.today, c.week, c.yesterday, c.prev_week);
            if c.fetched_at.elapsed().as_secs() >= LINES_CACHE_TTL_SECS {
                let db_path = state.db_path.clone();
                std::thread::spawn(move || refresh_lines_cache(&db_path));
            }
            vals
        } else {
            drop(cached);
            let db_path = state.db_path.clone();
            std::thread::spawn(move || refresh_lines_cache(&db_path));
            (0, 0, 0, 0)
        }
    };
    let agents_today = agents_today_count(&conn);

    let mut result = row;
    if let Some(obj) = result.as_object_mut() {
        obj.insert("today_lines_changed".to_string(), json!(today_lines));
        obj.insert("week_lines_changed".to_string(), json!(week_lines));
        obj.insert("yesterday_lines_changed".to_string(), json!(yesterday_lines));
        obj.insert("prev_week_lines_changed".to_string(), json!(prev_week_lines));
        obj.insert("agents_today".to_string(), json!(agents_today));
    }
    Ok(Json(result))
}

fn refresh_lines_cache(db_path: &std::path::Path) {
    if let Ok(conn) = Connection::open(db_path) {
        let vals = today_lines_changed(&conn);
        let mut cache = LINES_CACHE.lock().unwrap();
        *cache = Some(LinesCache {
            today: vals.0,
            week: vals.1,
            yesterday: vals.2,
            prev_week: vals.3,
            fetched_at: Instant::now(),
        });
    }
}

fn today_lines_changed(conn: &rusqlite::Connection) -> (i64, i64, i64, i64) {
    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let mut repos = vec![format!("{home}/.claude")];
    if let Ok(mut stmt) = conn.prepare(
        "SELECT DISTINCT path FROM projects WHERE path IS NOT NULL AND path != '' AND path != '.'",
    ) {
        if let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(0)) {
            for path in rows.flatten() {
                let expanded = if path.starts_with("~/") {
                    format!("{}/{}", home, &path[2..])
                } else {
                    path
                };
                if !expanded.is_empty() && std::path::Path::new(&expanded).join(".git").exists() {
                    repos.push(expanded);
                }
            }
        }
    }
    let mut today: i64 = 0;
    let mut week: i64 = 0;
    let mut yesterday: i64 = 0;
    let mut prev_week: i64 = 0;
    for repo in &repos {
        let periods: &[(&str, Option<&str>, usize)] = &[
            ("midnight", None, 0),                  // today
            ("1 week ago", None, 1),                // this week
            ("1 day ago", Some("midnight"), 2),     // yesterday only
            ("2 weeks ago", Some("1 week ago"), 3), // prev week
        ];
        for &(since, until, idx) in periods {
            let since_flag = format!("--since={since}");
            let until_flag = format!("--until={}", until.unwrap_or(""));
            let mut args = vec![
                "-C",
                repo,
                "log",
                "--all",
                &since_flag,
                "--shortstat",
                "--format=",
            ];
            if until.is_some() {
                args.push(&until_flag);
            }
            let out = Command::new("git").args(&args).output();
            if let Ok(o) = out {
                if o.status.success() {
                    let text = String::from_utf8_lossy(&o.stdout);
                    let mut sub: i64 = 0;
                    for line in text.lines() {
                        for part in line.split(',') {
                            let part = part.trim();
                            if part.contains("insertion") || part.contains("deletion") {
                                if let Some(n) = part
                                    .split_whitespace()
                                    .next()
                                    .and_then(|s| s.parse::<i64>().ok())
                                {
                                    sub += n;
                                }
                            }
                        }
                    }
                    match idx {
                        0 => today += sub,
                        1 => week += sub,
                        2 => yesterday += sub,
                        3 => prev_week += sub,
                        _ => {}
                    }
                }
            }
        }
    }
    (today, week, yesterday, prev_week)
}

pub fn agents_today_count(conn: &rusqlite::Connection) -> i64 {
    if let Ok(mut stmt) =
        conn.prepare("SELECT COUNT(*) FROM agent_runs WHERE date(started_at)=date('now')")
    {
        if let Ok(count) = stmt.query_row([], |r| r.get::<_, i64>(0)) {
            return count;
        }
    }
    0
}
