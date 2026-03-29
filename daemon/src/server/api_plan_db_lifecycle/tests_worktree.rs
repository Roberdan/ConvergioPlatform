use super::tests::setup_db;

#[test]
fn plan_complete_triggers_worktree_cleanup_fields() {
    use super::lifecycle_validation::worktree_cleanup_paths;

    let db = setup_db();
    let conn = db.connection();

    // Create a completed plan with worktree_path set
    conn.execute(
        "INSERT INTO plans (project_id, name, status, worktree_path) \
         VALUES ('test', 'Plan C', 'doing', '/tmp/convergio-plan-42')",
        [],
    )
    .unwrap();
    let plan_id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .unwrap();

    // Add waves with worktree paths
    conn.execute(
        "INSERT INTO waves (plan_id, wave_id, name, status, worktree_path, project_id) \
         VALUES (?1, 'W1', 'Wave 1', 'done', '/tmp/convergio-plan-42-w1', 'test'), \
                (?1, 'W2', 'Wave 2', 'done', '/tmp/convergio-plan-42-w2', 'test')",
        rusqlite::params![plan_id],
    )
    .unwrap();

    // Collect worktree paths that need cleanup
    let paths = worktree_cleanup_paths(conn, plan_id);
    assert_eq!(paths.len(), 3, "plan + 2 wave worktree paths");
    assert!(paths.contains(&"/tmp/convergio-plan-42".to_string()));
    assert!(paths.contains(&"/tmp/convergio-plan-42-w1".to_string()));
    assert!(paths.contains(&"/tmp/convergio-plan-42-w2".to_string()));
}

#[test]
fn plan_complete_no_worktree_returns_empty() {
    use super::lifecycle_validation::worktree_cleanup_paths;

    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO plans (project_id, name, status) VALUES ('test', 'Plan D', 'doing')",
        [],
    )
    .unwrap();
    let plan_id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .unwrap();

    let paths = worktree_cleanup_paths(conn, plan_id);
    assert!(paths.is_empty(), "no worktree paths when none set");
}
