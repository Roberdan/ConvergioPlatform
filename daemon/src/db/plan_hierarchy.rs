// Plan hierarchy: master plans, sub-plans, dependencies, execution modes.
// Project → Master Plan (is_master=1) → Plans (parent_plan_id) → Waves → Tasks.
//
// execution_mode on master plans:
//   "sequential" — children run in order (each depends on previous)
//   "parallel"   — all children can start immediately
//   "mixed"      — each child declares its own depends_on
//   "conditional" — children start only when depends_on plan meets condition

use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PlanNode {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub tasks_done: i64,
    pub tasks_total: i64,
    pub depends_on: Option<String>,
    pub execution_mode: Option<String>,
    pub is_master: bool,
    pub children: Vec<PlanNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectTree {
    pub project_id: String,
    pub project_name: String,
    pub plans: Vec<PlanNode>,
    pub total_tasks: i64,
    pub done_tasks: i64,
}

/// Fetch hierarchical plan tree for a project.
/// Returns master plans with their children nested, plus orphan plans.
pub fn project_plan_tree(conn: &Connection, project_id: &str) -> rusqlite::Result<ProjectTree> {
    let project_name = conn
        .query_row(
            "SELECT name FROM projects WHERE id = ?1",
            params![project_id],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_else(|_| project_id.to_string());

    let mut stmt = conn.prepare(
        "SELECT id, name, status, tasks_done, tasks_total, \
         depends_on, execution_mode, is_master, parent_plan_id \
         FROM plans WHERE project_id = ?1 \
         ORDER BY is_master DESC, id ASC",
    )?;

    let rows: Vec<(i64, String, String, i64, i64, Option<String>, Option<String>, bool, Option<i64>)> =
        stmt.query_map(params![project_id], |r| {
            Ok((
                r.get(0)?, r.get(1)?, r.get(2)?,
                r.get(3)?, r.get(4)?, r.get(5)?,
                r.get(6)?, r.get::<_, i64>(7).map(|v| v != 0)?,
                r.get(8)?,
            ))
        })?.filter_map(|r| r.ok()).collect();

    // Build tree: masters first, then attach children
    let mut masters: Vec<PlanNode> = Vec::new();
    let mut orphans: Vec<PlanNode> = Vec::new();

    // First pass: create all master plan nodes
    for row in &rows {
        if row.7 {
            masters.push(PlanNode {
                id: row.0, name: row.1.clone(), status: row.2.clone(),
                tasks_done: row.3, tasks_total: row.4,
                depends_on: row.5.clone(), execution_mode: row.6.clone(),
                is_master: true, children: Vec::new(),
            });
        }
    }

    // Second pass: attach children to masters, collect orphans
    for row in &rows {
        if row.7 { continue; } // skip masters
        let node = PlanNode {
            id: row.0, name: row.1.clone(), status: row.2.clone(),
            tasks_done: row.3, tasks_total: row.4,
            depends_on: row.5.clone(), execution_mode: row.6.clone(),
            is_master: false, children: Vec::new(),
        };
        if let Some(parent_id) = row.8 {
            if let Some(master) = masters.iter_mut().find(|m| m.id == parent_id) {
                master.children.push(node);
                continue;
            }
        }
        orphans.push(node);
    }

    // Merge: masters + orphans as top-level
    let mut all_plans = masters;
    all_plans.extend(orphans);

    let total_tasks: i64 = all_plans.iter().map(|p| sum_tasks_total(p)).sum();
    let done_tasks: i64 = all_plans.iter().map(|p| sum_tasks_done(p)).sum();

    Ok(ProjectTree {
        project_id: project_id.to_string(),
        project_name, plans: all_plans,
        total_tasks, done_tasks,
    })
}

/// Check if a plan's dependencies are satisfied (all done/cancelled).
pub fn dependencies_met(conn: &Connection, plan_id: i64) -> rusqlite::Result<bool> {
    let depends_on: Option<String> = conn.query_row(
        "SELECT depends_on FROM plans WHERE id = ?1",
        params![plan_id],
        |r| r.get(0),
    )?;

    let Some(deps) = depends_on else { return Ok(true) };
    if deps.trim().is_empty() { return Ok(true); }

    for dep_id_str in deps.split(',') {
        let dep_id: i64 = match dep_id_str.trim().parse() {
            Ok(id) => id,
            Err(_) => continue,
        };
        let status: String = conn.query_row(
            "SELECT status FROM plans WHERE id = ?1",
            params![dep_id],
            |r| r.get(0),
        )?;
        if status != "done" && status != "cancelled" {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Compute rollup status for a master plan from its children.
pub fn master_rollup(conn: &Connection, master_id: i64) -> rusqlite::Result<(i64, i64, String)> {
    let mut stmt = conn.prepare(
        "SELECT status, tasks_done, tasks_total FROM plans WHERE parent_plan_id = ?1",
    )?;
    let children: Vec<(String, i64, i64)> = stmt
        .query_map(params![master_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .filter_map(|r| r.ok())
        .collect();

    if children.is_empty() {
        return Ok((0, 0, "todo".to_string()));
    }

    let total: i64 = children.iter().map(|c| c.2).sum();
    let done: i64 = children.iter().map(|c| c.1).sum();

    let status = if children.iter().all(|c| c.0 == "done" || c.0 == "cancelled") {
        "done"
    } else if children.iter().any(|c| c.0 == "doing" || c.0 == "in_progress") {
        "doing"
    } else if children.iter().any(|c| c.0 == "blocked") {
        "blocked"
    } else {
        "todo"
    };

    Ok((done, total, status.to_string()))
}

fn sum_tasks_total(node: &PlanNode) -> i64 {
    node.tasks_total + node.children.iter().map(|c| sum_tasks_total(c)).sum::<i64>()
}

fn sum_tasks_done(node: &PlanNode) -> i64 {
    node.tasks_done + node.children.iter().map(|c| sum_tasks_done(c)).sum::<i64>()
}

#[cfg(test)]
mod tests {
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
        ).unwrap();
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
        conn.execute(
            "UPDATE plans SET depends_on = '4' WHERE id = 3", [],
        ).unwrap();
        // Plan 4 is 'doing', not done
        assert!(!dependencies_met(&conn, 3).unwrap());
    }

    #[test]
    fn master_rollup_computes_correctly() {
        let conn = setup_db();
        let (done, total, status) = master_rollup(&conn, 1).unwrap();
        assert_eq!(done, 5);
        assert_eq!(total, 8);
        assert_eq!(status, "todo"); // Child A done, Child B todo → no child "doing" → todo
    }

    #[test]
    fn dependencies_met_returns_true_when_no_deps() {
        let conn = setup_db();
        assert!(dependencies_met(&conn, 2).unwrap()); // no depends_on
    }
}
