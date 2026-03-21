use super::tracking::{log_usage, BudgetEntry, get_usage_for_period};
use super::status::{estimate_cost, estimate_task_cost, get_budget_status};
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
