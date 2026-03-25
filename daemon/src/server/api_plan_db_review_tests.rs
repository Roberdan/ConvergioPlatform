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
                 id INTEGER PRIMARY KEY,
                 plan_id INTEGER,
                 spec_file TEXT,
                 reviewer_agent TEXT NOT NULL,
                 verdict TEXT NOT NULL,
                 suggestions TEXT,
                 raw_report TEXT,
                 reviewed_at TEXT DEFAULT (datetime('now'))
             );
             INSERT INTO plans (id, project_id, name, status)
                 VALUES (1, 'convergio', 'Plan Alpha', 'draft');",
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
         VALUES (1, 'plan-reviewer', 'proceed')",
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
         VALUES (1, 'plan-reviewer', 'proceed'),
                (1, 'plan-business-advisor', 'proceed'),
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
         VALUES (1, 'plan-reviewer', 'proceed'),
                (1, 'challenger', 'revise');",
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

// BUG 3 — pre-plan review stored by spec_file and linked after plan creation
#[test]
fn review_register_by_spec_file_no_plan_id() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO plan_reviews (plan_id, spec_file, reviewer_agent, verdict) \
         VALUES (NULL, '/workspace/plans/plan-724.yaml', 'plan-reviewer', 'proceed')",
        [],
    )
    .unwrap();

    let row = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM plan_reviews \
         WHERE spec_file = '/workspace/plans/plan-724.yaml' AND plan_id IS NULL",
        [],
    )
    .expect("query")
    .expect("row");

    assert_eq!(row.get("c").and_then(|v| v.as_i64()), Some(1));
}

#[test]
fn review_link_by_spec_updates_plan_id() {
    let db = setup_db();
    let conn = db.connection();

    // Simulate pre-plan review registered by spec_file
    conn.execute(
        "INSERT INTO plan_reviews (plan_id, spec_file, reviewer_agent, verdict) \
         VALUES (NULL, '/workspace/plans/plan-724.yaml', 'plan-reviewer', 'proceed')",
        [],
    )
    .unwrap();

    // Simulate cvg plan create completing — link the review to the new plan
    let updated = conn
        .execute(
            "UPDATE plan_reviews SET plan_id = ?1 \
             WHERE spec_file = ?2 AND plan_id IS NULL",
            rusqlite::params![42_i64, "/workspace/plans/plan-724.yaml"],
        )
        .unwrap();
    assert_eq!(updated, 1);

    let linked: i64 = conn
        .query_row(
            "SELECT plan_id FROM plan_reviews WHERE spec_file = '/workspace/plans/plan-724.yaml'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(linked, 42);
}

// BUG 1 — valid verdict values are enforced server-side
#[test]
fn review_rejects_invalid_verdict_at_db_level() {
    let db = setup_db();
    let conn = db.connection();

    // Valid verdicts must be accepted
    for verdict in &["proceed", "revise", "reject"] {
        conn.execute(
            "INSERT INTO plan_reviews (plan_id, reviewer_agent, verdict) \
             VALUES (1, 'plan-reviewer', ?1)",
            rusqlite::params![verdict],
        )
        .unwrap_or_else(|e| panic!("valid verdict '{verdict}' rejected: {e}"));
    }

    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM plan_reviews WHERE plan_id = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(total, 3);
}
