// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for api_plan_db_import module.

use super::super::api_plan_db_import_parsers::parse_waves;
use crate::db::PlanDb;
use serde_json::json;

fn setup_db() -> PlanDb {
    let db = PlanDb::open_in_memory().expect("db");
    db.connection()
        .execute_batch(
            "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL);
             CREATE TABLE plans (
                 id INTEGER PRIMARY KEY, project_id TEXT NOT NULL,
                 name TEXT NOT NULL, status TEXT DEFAULT 'draft',
                 tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0,
                 updated_at TEXT
             );
             CREATE TABLE waves (
                 id INTEGER PRIMARY KEY, plan_id INTEGER, project_id TEXT,
                 wave_id TEXT, name TEXT, status TEXT DEFAULT 'pending',
                 position INTEGER DEFAULT 0, depends_on TEXT,
                 estimated_hours INTEGER DEFAULT 8,
                 tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0
             );
             CREATE TABLE tasks (
                 id INTEGER PRIMARY KEY, plan_id INTEGER, project_id TEXT,
                 wave_id_fk INTEGER, wave_id TEXT, task_id TEXT,
                 title TEXT, status TEXT DEFAULT 'pending',
                 priority TEXT, type TEXT, description TEXT,
                 test_criteria TEXT, model TEXT, assignee TEXT,
                 output_type TEXT, validator_agent TEXT,
                 effort_level INTEGER DEFAULT 1, notes TEXT
             );
             INSERT INTO projects (id, name) VALUES ('test', 'Test');
             INSERT INTO plans (id, project_id, name) VALUES (1, 'test', 'Plan A');",
        )
        .expect("schema");
    db
}

#[test]
fn plan_db_import_json_waves() {
    let body = json!({
        "plan_id": 1,
        "waves": [
            {
                "id": "W1",
                "name": "Wave 1",
                "tasks": [
                    {"id": "T1-01", "title": "Task 1", "priority": "P0"},
                    {"id": "T1-02", "title": "Task 2"}
                ]
            },
            {
                "id": "W2",
                "name": "Wave 2",
                "depends_on": "W1",
                "tasks": [
                    {"id": "T2-01", "title": "Task 3"}
                ]
            }
        ]
    });

    let waves = parse_waves(&body).expect("parse");
    assert_eq!(waves.len(), 2);
    assert_eq!(waves[0].tasks.len(), 2);
    assert_eq!(waves[1].tasks.len(), 1);
    assert_eq!(waves[1].depends_on.as_deref(), Some("W1"));
}

#[test]
fn plan_db_import_yaml_spec() {
    let yaml = "waves:\n  - id: W1\n    name: Wave 1\n    tasks:\n      - id: T1\n        title: First task\n";
    let body = json!({ "plan_id": 1, "spec": yaml });

    let waves = parse_waves(&body).expect("parse yaml");
    assert_eq!(waves.len(), 1);
    assert_eq!(waves[0].tasks[0].title, "First task");
}

#[test]
fn plan_db_import_creates_rows() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO waves (plan_id, project_id, wave_id, name, status, position, tasks_total) \
         VALUES (1, 'test', 'W1', 'Wave 1', 'pending', 0, 2)",
        [],
    )
    .unwrap();
    let wave_id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .unwrap();

    conn.execute(
        "INSERT INTO tasks (plan_id, project_id, wave_id_fk, wave_id, task_id, title, priority, type) \
         VALUES (1, 'test', ?1, 'W1', 'T1', 'Task 1', 'P0', 'feature'), \
                (1, 'test', ?1, 'W1', 'T2', 'Task 2', 'P1', 'feature')",
        rusqlite::params![wave_id],
    )
    .unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tasks WHERE plan_id = 1", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2);
}
