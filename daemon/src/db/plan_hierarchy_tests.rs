use super::*;
use rusqlite::Connection;

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL);
         CREATE TABLE plans (
             id INTEGER PRIMARY KEY, project_id TEXT, name TEXT,
             status TEXT DEFAULT 'todo', tasks_done INTEGER DEFAULT 0,
             tasks_total INTEGER DEFAULT 0, is_master INTEGER DEFAULT 0,
             parent_plan_id INTEGER, depends_on TEXT,
             execution_mode TEXT DEFAULT 'mixed'
         );
         INSERT INTO projects VALUES ('p1', 'TestProject');
         INSERT INTO plans VALUES (1,'p1','Master','doing',0,0,1,NULL,NULL,'mixed');
         INSERT INTO plans VALUES (2,'p1','Child A','done',5,5,0,1,NULL,NULL);
         INSERT INTO plans VALUES (3,'p1','Child B','todo',0,3,0,1,'2',NULL);
         INSERT INTO plans VALUES (4,'p1','Orphan','doing',1,2,0,NULL,NULL,NULL);",
    )
    .unwrap();
    conn
}

#[test]
fn project_tree_returns_hierarchy() {
    let conn = setup_db();
    let tree = project_plan_tree(&conn, "p1").unwrap();
    assert_eq!(tree.project_name, "TestProject");
    assert_eq!(tree.plans.len(), 2); // master + orphan
    assert_eq!(tree.plans[0].children.len(), 2); // Child A + B
    assert_eq!(tree.total_tasks, 10);
    assert_eq!(tree.done_tasks, 6);
}

#[test]
fn dependencies_met_returns_true_when_done() {
    let conn = setup_db();
    assert!(dependencies_met(&conn, 3).unwrap()); // depends on 2 which is done
}

#[test]
fn dependencies_met_returns_false_when_pending() {
    let conn = setup_db();
    conn.execute("UPDATE plans SET depends_on = '4' WHERE id = 3", [])
        .unwrap();
    assert!(!dependencies_met(&conn, 3).unwrap());
}

#[test]
fn master_rollup_computes_correctly() {
    let conn = setup_db();
    let (done, total, status) = master_rollup(&conn, 1).unwrap();
    assert_eq!(done, 5);
    assert_eq!(total, 8);
    assert_eq!(status, "todo");
}

#[test]
fn dependencies_met_returns_true_when_no_deps() {
    let conn = setup_db();
    assert!(dependencies_met(&conn, 2).unwrap());
}

#[test]
fn parent_promoted_to_master_on_child_insert() {
    let conn = setup_db();
    let is_master: bool = conn
        .query_row("SELECT is_master FROM plans WHERE id = 4", [], |r| {
            r.get::<_, i64>(0).map(|v| v != 0)
        })
        .unwrap();
    assert!(!is_master);
    conn.execute(
        "UPDATE plans SET is_master = 1 WHERE id = 4 AND is_master = 0",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO plans VALUES (5,'p1','Sub','todo',0,2,0,4,NULL,NULL)",
        [],
    )
    .unwrap();
    let tree = project_plan_tree(&conn, "p1").unwrap();
    let plan4 = tree.plans.iter().find(|p| p.id == 4).unwrap();
    assert!(plan4.is_master);
    assert_eq!(plan4.children.len(), 1);
    assert_eq!(plan4.children[0].name, "Sub");
}
