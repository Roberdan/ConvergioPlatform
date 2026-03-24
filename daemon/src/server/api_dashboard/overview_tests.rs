use super::*;
use std::sync::Mutex;

// Serialize semaphore tests to avoid cross-test interference on the shared static
static TEST_LOCK: Mutex<()> = Mutex::new(());

/// Semaphore must be initialized with exactly 4 permits (max concurrent refreshes)
#[test]
fn refresh_semaphore_has_four_permits() {
    let _guard = TEST_LOCK.lock().unwrap();
    let available = REFRESH_SEMAPHORE.available_permits();
    assert_eq!(
        available, 4,
        "REFRESH_SEMAPHORE must allow exactly 4 concurrent refreshes"
    );
}

/// try_acquire must return a permit when none are held (skip-if-busy logic)
#[test]
fn refresh_semaphore_try_acquire_succeeds_when_free() {
    let _guard = TEST_LOCK.lock().unwrap();
    let permit = REFRESH_SEMAPHORE.try_acquire();
    assert!(
        permit.is_ok(),
        "try_acquire should succeed on a free semaphore"
    );
}

/// try_acquire must fail once all 4 permits are exhausted (skip-refresh-when-busy path)
#[test]
fn refresh_semaphore_try_acquire_fails_when_full() {
    let _guard = TEST_LOCK.lock().unwrap();
    // Drain all permits
    let _p1 = REFRESH_SEMAPHORE.try_acquire().unwrap();
    let _p2 = REFRESH_SEMAPHORE.try_acquire().unwrap();
    let _p3 = REFRESH_SEMAPHORE.try_acquire().unwrap();
    let _p4 = REFRESH_SEMAPHORE.try_acquire().unwrap();
    // 5th must fail — this is the skip condition
    let fifth = REFRESH_SEMAPHORE.try_acquire();
    assert!(
        fifth.is_err(),
        "try_acquire should fail when all 4 permits are held"
    );
    // permits released automatically via Drop when _p1.._p4 go out of scope
}
