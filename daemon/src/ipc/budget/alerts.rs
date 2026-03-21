// T8064: Budget alerts + T8065: Alert integration
use rusqlite::{params, Connection};
use serde::Serialize;

use super::status::get_budget_status;

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
    use crate::ipc::budget::tracking::{log_usage, BudgetEntry};
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
