// T8060: Core usage tracking
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

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
