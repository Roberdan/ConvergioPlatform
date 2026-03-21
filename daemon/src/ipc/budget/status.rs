// T8061: Budget status + T8063: Cost estimation
use rusqlite::{params, Connection};
use serde::Serialize;

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

// T8063: Cost estimation

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
