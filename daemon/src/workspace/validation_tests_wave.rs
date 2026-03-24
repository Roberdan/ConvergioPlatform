use super::*;

// --- validate_wave ---

#[test]
fn validate_wave_batch_promotes_submitted_to_done() {
    let pool = make_pool();
    insert_plan(&pool, 5, 0, 3);
    let wave_db_id = insert_wave(&pool, 50, 5, 1, "in_progress");
    insert_task(&pool, 5, wave_db_id, "submitted");
    insert_task(&pool, 5, wave_db_id, "submitted");
    insert_task(&pool, 5, wave_db_id, "done"); // already done — stamp validated_at
    pool.get()
        .unwrap()
        .execute(
            "UPDATE tasks SET validated_at = datetime('now'), validated_by = 'thor'
             WHERE wave_id_fk = ?1 AND status = 'done'",
            rusqlite::params![wave_db_id],
        )
        .unwrap();

    let result = validate_wave(wave_db_id, "thor-per-wave", &pool);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
    let r = result.unwrap();
    assert_eq!(r.tasks_validated, 2);
    assert_eq!(r.wave_status, "done");

    let missing_count: i64 = pool
        .get()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE wave_id_fk = ?1 AND (status != 'done' OR validated_at IS NULL)",
            rusqlite::params![wave_db_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(missing_count, 0, "all tasks must be done with validated_at");
}

#[test]
fn validate_wave_blocks_on_unresolved_tasks() {
    let pool = make_pool();
    insert_plan(&pool, 6, 0, 2);
    let wave_db_id = insert_wave(&pool, 60, 6, 1, "in_progress");
    insert_task(&pool, 6, wave_db_id, "submitted");
    insert_task(&pool, 6, wave_db_id, "in_progress"); // unresolved

    let result = validate_wave(wave_db_id, "thor-per-wave", &pool);
    assert!(
        result.is_err(),
        "expected Err for unresolved in_progress task"
    );
    assert!(
        result.unwrap_err().to_string().contains("unresolved"),
        "error should mention unresolved tasks"
    );
}

// --- check_wave_sequential ---

#[test]
fn check_wave_sequential_allows_first_wave() {
    let pool = make_pool();
    insert_plan(&pool, 7, 0, 0);
    let _wave_db_id = insert_wave(&pool, 70, 7, 1, "pending");

    let result = check_wave_sequential(7, 1, &pool);
    assert!(result.is_ok(), "first wave should always be allowed");
}

#[test]
fn check_wave_sequential_allows_when_predecessors_done() {
    let pool = make_pool();
    insert_plan(&pool, 8, 0, 0);
    insert_wave(&pool, 80, 8, 1, "done");
    insert_wave(&pool, 81, 8, 2, "pending");

    let result = check_wave_sequential(8, 2, &pool);
    assert!(result.is_ok(), "should allow wave 2 when wave 1 is done");
}

#[test]
fn check_wave_sequential_blocks_when_predecessor_not_done() {
    let pool = make_pool();
    insert_plan(&pool, 9, 0, 0);
    insert_wave(&pool, 90, 9, 1, "in_progress");
    insert_wave(&pool, 91, 9, 2, "pending");

    let result = check_wave_sequential(9, 2, &pool);
    assert!(
        result.is_err(),
        "should block wave 2 when wave 1 is in_progress"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("must be completed"),
        "error should mention completion requirement, got: {err}"
    );
}

#[test]
fn check_wave_sequential_blocks_when_predecessor_merging() {
    let pool = make_pool();
    insert_plan(&pool, 10, 0, 0);
    insert_wave(&pool, 100, 10, 1, "merging");
    insert_wave(&pool, 101, 10, 2, "pending");

    let result = check_wave_sequential(10, 2, &pool);
    assert!(
        result.is_err(),
        "merging status is not terminal — should block"
    );
}
