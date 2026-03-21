use super::{gethostname, AcquireResult, IpcLockStore};

#[test]
fn acquire_lock_succeeds() {
    let mut store = IpcLockStore::open_in_memory().unwrap();
    let result = store
        .acquire_lock("src/*.rs", "agent-a", "host1", 1000)
        .unwrap();
    assert_eq!(result, AcquireResult::Acquired);
}

#[test]
fn acquire_same_pattern_different_agent_rejected() {
    let mut store = IpcLockStore::open_in_memory().unwrap();
    store
        .acquire_lock("src/*.rs", "agent-a", "host1", 1000)
        .unwrap();
    let result = store
        .acquire_lock("src/*.rs", "agent-b", "host1", 2000)
        .unwrap();
    match result {
        AcquireResult::Rejected(lock) => {
            assert_eq!(lock.agent, "agent-a");
            assert_eq!(lock.file_pattern, "src/*.rs");
        }
        AcquireResult::Acquired => panic!("expected rejection"),
    }
}

#[test]
fn acquire_same_agent_same_host_reacquires() {
    let mut store = IpcLockStore::open_in_memory().unwrap();
    store
        .acquire_lock("src/*.rs", "agent-a", "host1", 1000)
        .unwrap();
    let result = store
        .acquire_lock("src/*.rs", "agent-a", "host1", 1001)
        .unwrap();
    assert_eq!(result, AcquireResult::Acquired);
    let locks = store.list_locks().unwrap();
    assert_eq!(locks.len(), 1);
    assert_eq!(locks[0].pid, 1001);
}

#[test]
fn release_lock_works() {
    let mut store = IpcLockStore::open_in_memory().unwrap();
    store
        .acquire_lock("src/*.rs", "agent-a", "host1", 1000)
        .unwrap();
    let released = store.release_lock("src/*.rs", "agent-a", "host1").unwrap();
    assert_eq!(released, 1);
    let locks = store.list_locks().unwrap();
    assert!(locks.is_empty());
}

#[test]
fn list_locks_returns_all() {
    let mut store = IpcLockStore::open_in_memory().unwrap();
    store
        .acquire_lock("src/*.rs", "agent-a", "host1", 1000)
        .unwrap();
    store
        .acquire_lock("tests/*.rs", "agent-b", "host2", 2000)
        .unwrap();
    let locks = store.list_locks().unwrap();
    assert_eq!(locks.len(), 2);
}

#[test]
fn prune_dead_removes_stale_pids() {
    let mut store = IpcLockStore::open_in_memory().unwrap();
    // PID 999999999 almost certainly doesn't exist
    store
        .acquire_lock("src/*.rs", "agent-a", &gethostname(), 999_999_999)
        .unwrap();
    let pruned = store.prune_dead().unwrap();
    assert_eq!(pruned, 1);
    assert!(store.list_locks().unwrap().is_empty());
}
