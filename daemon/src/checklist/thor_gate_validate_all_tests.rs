// Thor gate validate_all tests extracted from thor_gate_tests.rs.
// Why: keep thor_gate_tests.rs ≤250 lines per CONSTITUTION Article V.
use crate::checklist::thor_gate::ChecklistGate;

#[test]
fn validate_all_empty_registry_returns_empty_vec() {
    use crate::checklist::registry::ChecklistRegistry;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let registry = ChecklistRegistry::load_directory(dir.path()).unwrap();
    let gate = ChecklistGate::new();
    let results = gate.validate_all(&registry);

    assert!(results.is_empty(), "empty registry must return empty results");
}

#[test]
fn validate_all_returns_one_result_per_checklist() {
    use crate::checklist::registry::ChecklistRegistry;
    use std::fs;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    // Write two minimal YAML checklists.
    let yaml_a = r#"
name: alpha
version: "1.0.0"
mode: do-confirm
items:
  - id: a1
    title: "Alpha step"
    command: "true"
    expected: ""
    severity: info
"#;
    let yaml_b = r#"
name: beta
version: "1.0.0"
mode: do-confirm
items:
  - id: b1
    title: "Beta step"
    command: "true"
    expected: ""
    severity: info
"#;
    fs::write(dir.path().join("alpha.yaml"), yaml_a).unwrap();
    fs::write(dir.path().join("beta.yaml"), yaml_b).unwrap();

    let registry = ChecklistRegistry::load_directory(dir.path()).unwrap();
    let gate = ChecklistGate::new();
    let results = gate.validate_all(&registry);

    assert_eq!(results.len(), 2, "one result per checklist in registry");
}
