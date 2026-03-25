use std::path::Path;

#[test]
fn escalation_metric_collector_exists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    assert!(
        root.join("evolution/adapters/escalation-collector.ts").exists(),
        "escalation-collector.ts must exist in evolution/adapters/"
    );
}

#[test]
fn escalation_metric_test_file_exists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    assert!(
        root.join("evolution/adapters/escalation-collector.test.ts").exists(),
        "escalation-collector.test.ts must exist"
    );
}
