use super::*;

// --- verify generation ---

#[test]
fn defaults_verify_generates_test_f_per_file() {
    let mut task = make_task("feature");
    task.files = vec!["src/main.rs".to_string(), "src/lib.rs".to_string()];
    apply_defaults(&mut task);
    assert_eq!(
        task.verify,
        vec!["test -f src/main.rs", "test -f src/lib.rs"]
    );
}

#[test]
fn defaults_verify_not_overridden_when_present() {
    let mut task = make_task("feature");
    task.files = vec!["src/main.rs".to_string()];
    task.verify = vec!["cargo check".to_string()];
    apply_defaults(&mut task);
    assert_eq!(task.verify, vec!["cargo check"]);
}

#[test]
fn defaults_verify_skipped_when_no_files() {
    let mut task = make_task("feature");
    apply_defaults(&mut task);
    assert!(task.verify.is_empty());
}

// --- effort inference ---

#[test]
fn defaults_effort_no_files_gives_level_2() {
    let mut task = make_task("feature");
    apply_defaults(&mut task);
    assert_eq!(task.effort_level, Some(2));
}

#[test]
fn defaults_effort_one_file_gives_level_1() {
    let mut task = make_task("feature");
    task.files = vec!["src/main.rs".to_string()];
    apply_defaults(&mut task);
    assert_eq!(task.effort_level, Some(1));
}

#[test]
fn defaults_effort_three_files_gives_level_2() {
    let mut task = make_task("feature");
    task.files = (0..3).map(|i| format!("src/f{i}.rs")).collect();
    apply_defaults(&mut task);
    assert_eq!(task.effort_level, Some(2));
}

#[test]
fn defaults_effort_five_files_gives_level_3() {
    let mut task = make_task("feature");
    task.files = (0..5).map(|i| format!("src/f{i}.rs")).collect();
    apply_defaults(&mut task);
    assert_eq!(task.effort_level, Some(3));
}

#[test]
fn defaults_effort_planning_with_no_files_gives_level_2() {
    let mut task = make_task("planning");
    apply_defaults(&mut task);
    assert_eq!(task.effort_level, Some(2));
}

#[test]
fn defaults_effort_chore_five_files_gives_level_3() {
    let mut task = make_task("chore");
    task.files = (0..5).map(|i| format!("f{i}")).collect();
    apply_defaults(&mut task);
    assert_eq!(task.effort_level, Some(3));
}

#[test]
fn defaults_effort_explicit_not_overridden() {
    let mut task = make_task("feature");
    task.effort_level = Some(3);
    apply_defaults(&mut task);
    assert_eq!(task.effort_level, Some(3));
}

// --- test_criteria from verify ---

#[test]
fn defaults_test_criteria_from_verify_when_absent() {
    let mut task = make_task("feature");
    task.verify = vec!["cargo test -- foo".to_string(), "cargo check".to_string()];
    apply_defaults(&mut task);
    let criteria = task.test_criteria.expect("test_criteria should be set");
    let s = criteria.as_str().expect("should be string");
    assert!(s.contains("cargo test -- foo"), "criteria={s}");
    assert!(s.contains("cargo check"), "criteria={s}");
}

#[test]
fn defaults_test_criteria_not_overridden_when_present() {
    let mut task = make_task("feature");
    task.verify = vec!["cargo test".to_string()];
    task.test_criteria = Some(serde_json::Value::String("custom criteria".to_string()));
    apply_defaults(&mut task);
    let s = task.test_criteria.unwrap();
    assert_eq!(s.as_str().unwrap(), "custom criteria");
}

#[test]
fn defaults_test_criteria_empty_when_no_verify() {
    let mut task = make_task("feature");
    apply_defaults(&mut task);
    assert!(task.test_criteria.is_none());
}
