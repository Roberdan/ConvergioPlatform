use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

// --- T8060: Core tracking ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetEntry {
    pub subscription: String,
    pub date: String,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub estimated_cost_usd: f64,
    pub model: String,
    pub task_ref: String,
}

pub fn log_usage(conn: &Connection, entry: &BudgetEntry) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO ipc_budget_log (subscription, date, tokens_in, tokens_out, estimated_cost_usd, model, task_ref)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![entry.subscription, entry.date, entry.tokens_in, entry.tokens_out,
                entry.estimated_cost_usd, entry.model, entry.task_ref],
    )?;
    Ok(())
}

pub fn get_usage_for_period(
    conn: &Connection,
    subscription: &str,
    from: &str,
    to: &str,
) -> rusqlite::Result<Vec<BudgetEntry>> {
    let mut stmt = conn.prepare(
        "SELECT subscription, date, tokens_in, tokens_out, estimated_cost_usd, model, task_ref
         FROM ipc_budget_log WHERE subscription=?1 AND date BETWEEN ?2 AND ?3 ORDER BY date",
    )?;
    let rows = stmt.query_map(params![subscription, from, to], |row| {
        Ok(BudgetEntry {
            subscription: row.get(0)?,
            date: row.get(1)?,
            tokens_in: row.get(2)?,
            tokens_out: row.get(3)?,
            estimated_cost_usd: row.get(4)?,
            model: row.get(5)?,
            task_ref: row.get(6)?,
        })
    })?;
    rows.collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct DailySummary {
    pub date: String,
    pub total_tokens_in: i64,
    pub total_tokens_out: i64,
    pub total_cost: f64,
}

pub fn get_daily_summary(
    conn: &Connection,
    subscription: &str,
) -> rusqlite::Result<Vec<DailySummary>> {
    let mut stmt = conn.prepare(
        "SELECT date, SUM(tokens_in), SUM(tokens_out), SUM(estimated_cost_usd)
         FROM ipc_budget_log WHERE subscription=?1 GROUP BY date ORDER BY date DESC LIMIT 30",
    )?;
    let rows = stmt.query_map(params![subscription], |row| {
        Ok(DailySummary {
            date: row.get(0)?,
            total_tokens_in: row.get(1)?,
            total_tokens_out: row.get(2)?,
            total_cost: row.get(3)?,
        })
    })?;
    rows.collect()
}

// --- T8061: Budget status ---

#[derive(Debug, Clone, Serialize)]
pub struct BudgetStatus {
    pub subscription: String,
    pub budget_usd: f64,
    pub total_spent: f64,
    pub remaining_budget: f64,
    pub days_remaining: i32,
    pub daily_avg: f64,
    pub projected_total: f64,
    pub usage_pct: f64,
}

pub fn get_budget_status(
    conn: &Connection,
    subscription: &str,
) -> rusqlite::Result<Option<BudgetStatus>> {
    let sub_row = conn.query_row(
        "SELECT budget_usd, reset_day FROM ipc_subscriptions WHERE name=?1",
        params![subscription],
        |row| Ok((row.get::<_, f64>(0)?, row.get::<_, i32>(1)?)),
    );
    let (budget_usd, reset_day) = match sub_row {
        Ok(r) => r,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(e) => return Err(e),
    };
    let total_spent: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(estimated_cost_usd), 0.0) FROM ipc_budget_log WHERE subscription=?1",
            params![subscription],
            |r| r.get(0),
        )
        .unwrap_or(0.0);
    let day_count: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT date) FROM ipc_budget_log WHERE subscription=?1",
            params![subscription],
            |r| r.get(0),
        )
        .unwrap_or(1);
    let daily_avg = if day_count > 0 {
        total_spent / day_count as f64
    } else {
        0.0
    };
    let days_remaining = (reset_day - 1).max(1);
    let projected_total = total_spent + daily_avg * days_remaining as f64;
    let remaining = budget_usd - total_spent;
    let usage_pct = if budget_usd > 0.0 {
        (total_spent / budget_usd) * 100.0
    } else {
        0.0
    };
    Ok(Some(BudgetStatus {
        subscription: subscription.to_string(),
        budget_usd,
        total_spent,
        remaining_budget: remaining,
        days_remaining,
        daily_avg,
        projected_total,
        usage_pct,
    }))
}

// --- T8063: Cost estimation ---

#[derive(Debug, Clone, Serialize)]
pub struct ModelPricing {
    pub model: String,
    pub input_cost_per_1k: f64,
    pub output_cost_per_1k: f64,
}

pub fn default_pricing() -> Vec<ModelPricing> {
    vec![
        ModelPricing {
            model: "gpt-4o".into(),
            input_cost_per_1k: 0.005,
            output_cost_per_1k: 0.015,
        },
        ModelPricing {
            model: "gpt-4o-mini".into(),
            input_cost_per_1k: 0.00015,
            output_cost_per_1k: 0.0006,
        },
        ModelPricing {
            model: "claude-sonnet".into(),
            input_cost_per_1k: 0.003,
            output_cost_per_1k: 0.015,
        },
        ModelPricing {
            model: "claude-opus".into(),
            input_cost_per_1k: 0.015,
            output_cost_per_1k: 0.075,
        },
        ModelPricing {
            model: "claude-haiku".into(),
            input_cost_per_1k: 0.00025,
            output_cost_per_1k: 0.00125,
        },
    ]
}

pub static DEFAULT_PRICING: std::sync::LazyLock<Vec<ModelPricing>> =
    std::sync::LazyLock::new(default_pricing);

pub fn estimate_cost(model: &str, tokens_in: i64, tokens_out: i64) -> f64 {
    let pricing = DEFAULT_PRICING
        .iter()
        .find(|p| model.contains(&p.model))
        .cloned()
        .unwrap_or(ModelPricing {
            model: model.to_string(),
            input_cost_per_1k: 0.003,
            output_cost_per_1k: 0.015,
        });
    (tokens_in as f64 / 1000.0) * pricing.input_cost_per_1k
        + (tokens_out as f64 / 1000.0) * pricing.output_cost_per_1k
}

pub fn estimate_task_cost(task_text: &str, model: &str) -> f64 {
    let words = task_text.split_whitespace().count();
    let est_tokens_in = (words as f64 * 1.3) as i64;
    let est_tokens_out = est_tokens_in * 3;
    estimate_cost(model, est_tokens_in, est_tokens_out)
}

// --- T8064: Budget alerts ---

#[derive(Debug, Clone, Serialize)]
pub struct BudgetAlert {
    pub subscription: String,
    pub level: AlertLevel,
    pub usage_pct: f64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum AlertLevel {
    Warning,
    High,
    Critical,
}

pub fn check_budget_thresholds(
    conn: &Connection,
    subscription: &str,
) -> rusqlite::Result<Option<BudgetAlert>> {
    let status = match get_budget_status(conn, subscription)? {
        Some(s) => s,
        None => return Ok(None),
    };
    let pct = status.usage_pct;
    if pct >= 95.0 {
        Ok(Some(BudgetAlert {
            subscription: subscription.to_string(),
            level: AlertLevel::Critical,
            usage_pct: pct,
            message: format!("CRITICAL: {subscription} at {pct:.0}% — budget nearly exhausted"),
        }))
    } else if pct >= 85.0 {
        Ok(Some(BudgetAlert {
            subscription: subscription.to_string(),
            level: AlertLevel::High,
            usage_pct: pct,
            message: format!("HIGH: {subscription} at {pct:.0}% — approaching limit"),
        }))
    } else if pct >= 70.0 {
        Ok(Some(BudgetAlert {
            subscription: subscription.to_string(),
            level: AlertLevel::Warning,
            usage_pct: pct,
            message: format!("WARNING: {subscription} at {pct:.0}% — monitor spending"),
        }))
    } else {
        Ok(None)
    }
}

// --- T8065: Alert integration ---

pub fn publish_budget_alert(conn: &Connection, alert: &BudgetAlert) -> rusqlite::Result<()> {
    let payload = serde_json::to_string(alert).unwrap_or_default();
    conn.execute(
        "INSERT INTO notifications (title, body, category, created_at)
         VALUES (?1, ?2, 'ipc_budget_alert', datetime('now'))",
        params![format!("Budget Alert: {}", alert.subscription), payload,],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("
            CREATE TABLE ipc_budget_log (id INTEGER PRIMARY KEY, subscription TEXT, date TEXT, tokens_in INTEGER, tokens_out INTEGER, estimated_cost_usd REAL, model TEXT, task_ref TEXT);
            CREATE TABLE ipc_subscriptions (name TEXT PRIMARY KEY, provider TEXT, plan TEXT, budget_usd REAL, reset_day INTEGER, models TEXT);
            CREATE TABLE notifications (id INTEGER PRIMARY KEY, title TEXT, body TEXT, category TEXT, created_at TEXT);
        ").unwrap();
        conn
    }

    #[test]
    fn test_log_usage() {
        let conn = setup_db();
        log_usage(
            &conn,
            &BudgetEntry {
                subscription: "sub1".into(),
                date: "2026-03-16".into(),
                tokens_in: 1000,
                tokens_out: 500,
                estimated_cost_usd: 0.05,
                model: "gpt-4o".into(),
                task_ref: "t1".into(),
            },
        )
        .unwrap();
        let rows = get_usage_for_period(&conn, "sub1", "2026-03-01", "2026-03-31").unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_budget_status_calculation() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO ipc_subscriptions VALUES ('sub1','openai','pro',100.0,30,'[]')",
            [],
        )
        .unwrap();
        log_usage(
            &conn,
            &BudgetEntry {
                subscription: "sub1".into(),
                date: "2026-03-15".into(),
                tokens_in: 10000,
                tokens_out: 5000,
                estimated_cost_usd: 50.0,
                model: "gpt-4o".into(),
                task_ref: "".into(),
            },
        )
        .unwrap();
        let st = get_budget_status(&conn, "sub1").unwrap().unwrap();
        assert_eq!(st.total_spent, 50.0);
        assert_eq!(st.remaining_budget, 50.0);
        assert!((st.usage_pct - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_estimate_cost() {
        let cost = estimate_cost("gpt-4o", 1000, 1000);
        assert!(cost > 0.0);
    }

    #[test]
    fn test_estimate_task_cost() {
        let cost = estimate_task_cost("write a unit test for the auth module", "gpt-4o");
        assert!(cost > 0.0);
    }

    #[test]
    fn test_thresholds_at_69_pct() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO ipc_subscriptions VALUES ('s','p','pl',100.0,30,'[]')",
            [],
        )
        .unwrap();
        log_usage(
            &conn,
            &BudgetEntry {
                subscription: "s".into(),
                date: "2026-03-16".into(),
                tokens_in: 100,
                tokens_out: 100,
                estimated_cost_usd: 69.0,
                model: "m".into(),
                task_ref: "".into(),
            },
        )
        .unwrap();
        let alert = check_budget_thresholds(&conn, "s").unwrap();
        assert!(alert.is_none()); // 69% < 70 threshold
    }

    #[test]
    fn test_thresholds_at_71_pct() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO ipc_subscriptions VALUES ('s','p','pl',100.0,30,'[]')",
            [],
        )
        .unwrap();
        log_usage(
            &conn,
            &BudgetEntry {
                subscription: "s".into(),
                date: "2026-03-16".into(),
                tokens_in: 100,
                tokens_out: 100,
                estimated_cost_usd: 71.0,
                model: "m".into(),
                task_ref: "".into(),
            },
        )
        .unwrap();
        let alert = check_budget_thresholds(&conn, "s").unwrap().unwrap();
        assert_eq!(alert.level, AlertLevel::Warning);
    }

    #[test]
    fn test_thresholds_at_96_pct() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO ipc_subscriptions VALUES ('s','p','pl',100.0,30,'[]')",
            [],
        )
        .unwrap();
        log_usage(
            &conn,
            &BudgetEntry {
                subscription: "s".into(),
                date: "2026-03-16".into(),
                tokens_in: 100,
                tokens_out: 100,
                estimated_cost_usd: 96.0,
                model: "m".into(),
                task_ref: "".into(),
            },
        )
        .unwrap();
        let alert = check_budget_thresholds(&conn, "s").unwrap().unwrap();
        assert_eq!(alert.level, AlertLevel::Critical);
    }
}
