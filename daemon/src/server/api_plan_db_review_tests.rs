// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for api_plan_db_review module.

use crate::db::PlanDb;
use crate::server::state::query_one;

fn setup_db() -> PlanDb {
    let db = PlanDb::open_in_memory().expect("db");
    db.connection()
        .execute_batch(
            "CREATE TABLE plans (
                 id INTEGER PRIMARY KEY, project_id TEXT, name TEXT, status TEXT
             );
             CREATE TABLE plan_reviews (
                 id INTEGER PRIMARY KEY, plan_id INTEGER, reviewer_agent TEXT,
                 verdict TEXT, suggestions TEXT, raw_report TEXT,
                 reviewed_at TEXT DEFAULT (datetime('now'))
             );
             INSERT INTO plans (id, project_id, name, status)
                 VALUES (1, 'test', 'Test Plan', 'draft');",
        )
        .expect("schema");
    db
}

#[test]
fn review_register_inserts_row() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO plan_reviews (plan_id, reviewer_agent, verdict) \
         VALUES (1, 'plan-reviewer', 'approved')",
        [],
    )
    .unwrap();

    let row = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM plan_reviews WHERE plan_id = 1",
        [],
    )
    .expect("query")
    .expect("row");

    assert_eq!(row.get("c").and_then(|v| v.as_i64()), Some(1));
}

#[test]
fn review_check_counts_by_type() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute_batch(
        "INSERT INTO plan_reviews (plan_id, reviewer_agent, verdict)
         VALUES (1, 'plan-reviewer', 'approved'),
                (1, 'plan-business-advisor', 'approved'),
                (1, 'challenger', 'proceed');",
    )
    .unwrap();

    let reviewer: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM plan_reviews \
             WHERE plan_id = 1 AND reviewer_agent LIKE '%reviewer%' \
             AND reviewer_agent NOT LIKE '%business%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(reviewer, 1);

    let business: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM plan_reviews \
             WHERE plan_id = 1 AND (reviewer_agent LIKE '%business%' \
               OR reviewer_agent LIKE '%advisor%')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(business, 1);
}

#[test]
fn review_reset_deletes_all_for_plan() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute_batch(
        "INSERT INTO plan_reviews (plan_id, reviewer_agent, verdict)
         VALUES (1, 'plan-reviewer', 'approved'),
                (1, 'challenger', 'proceed');",
    )
    .unwrap();

    let deleted = conn
        .execute("DELETE FROM plan_reviews WHERE plan_id = 1", [])
        .unwrap();
    assert_eq!(deleted, 2);

    let remaining: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM plan_reviews WHERE plan_id = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(remaining, 0);
}
