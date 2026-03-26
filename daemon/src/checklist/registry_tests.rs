#[cfg(test)]
mod tests {
    use std::path::Path;
    use tempfile::TempDir;

    use crate::checklist::engine::{CheckMode, CheckSeverity};
    use crate::checklist::registry::ChecklistRegistry;

    fn write_yaml(dir: &TempDir, filename: &str, content: &str) {
        let path = dir.path().join(filename);
        std::fs::write(path, content).expect("failed to write yaml fixture");
    }

    const DEPLOYMENT_YAML: &str = r#"
name: deployment-checklist
version: "1.0.0"
mode: do-confirm
items:
  - id: check-tests
    title: All tests pass
    command: "echo PASS"
    expected: "PASS"
    severity: critical
    depends_on: []
  - id: check-lint
    title: No lint errors
    command: "echo CLEAN"
    expected: "CLEAN"
    severity: warning
    depends_on: [check-tests]
"#;

    const SECURITY_YAML: &str = r#"
name: security-checklist
version: "2.1.0"
mode: read-do
items:
  - id: scan-deps
    title: Scan dependencies
    command: "echo SECURE"
    expected: "SECURE"
    severity: critical
    depends_on: []
"#;

    #[test]
    fn load_directory_parses_single_yaml() {
        let dir = TempDir::new().unwrap();
        write_yaml(&dir, "deployment.yaml", DEPLOYMENT_YAML);

        let registry = ChecklistRegistry::load_directory(dir.path()).unwrap();

        let cl = registry.get("deployment-checklist").expect("checklist not found");
        assert_eq!(cl.name, "deployment-checklist");
        assert_eq!(cl.version, "1.0.0");
        assert!(matches!(cl.mode, CheckMode::DoConfirm));
        assert_eq!(cl.items.len(), 2);
    }

    #[test]
    fn load_directory_parses_item_fields() {
        let dir = TempDir::new().unwrap();
        write_yaml(&dir, "deployment.yaml", DEPLOYMENT_YAML);

        let registry = ChecklistRegistry::load_directory(dir.path()).unwrap();
        let cl = registry.get("deployment-checklist").unwrap();

        let first = &cl.items[0];
        assert_eq!(first.id, "check-tests");
        assert_eq!(first.title, "All tests pass");
        assert_eq!(first.command, "echo PASS");
        assert_eq!(first.expected, "PASS");
        assert!(matches!(first.severity, CheckSeverity::Critical));
        assert!(first.depends_on.is_empty());

        let second = &cl.items[1];
        assert!(matches!(second.severity, CheckSeverity::Warning));
        assert_eq!(second.depends_on, vec!["check-tests"]);
    }

    #[test]
    fn load_directory_multiple_files() {
        let dir = TempDir::new().unwrap();
        write_yaml(&dir, "deployment.yaml", DEPLOYMENT_YAML);
        write_yaml(&dir, "security.yaml", SECURITY_YAML);

        let registry = ChecklistRegistry::load_directory(dir.path()).unwrap();

        assert!(registry.get("deployment-checklist").is_some());
        assert!(registry.get("security-checklist").is_some());
        assert_eq!(registry.list().len(), 2);
    }

    #[test]
    fn load_directory_read_do_mode() {
        let dir = TempDir::new().unwrap();
        write_yaml(&dir, "security.yaml", SECURITY_YAML);

        let registry = ChecklistRegistry::load_directory(dir.path()).unwrap();
        let cl = registry.get("security-checklist").unwrap();

        assert!(matches!(cl.mode, CheckMode::ReadDo));
    }

    #[test]
    fn load_directory_empty_dir_returns_empty_registry() {
        let dir = TempDir::new().unwrap();
        let registry = ChecklistRegistry::load_directory(dir.path()).unwrap();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn get_returns_none_for_unknown_name() {
        let dir = TempDir::new().unwrap();
        write_yaml(&dir, "deployment.yaml", DEPLOYMENT_YAML);

        let registry = ChecklistRegistry::load_directory(dir.path()).unwrap();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn load_directory_ignores_non_yaml_files() {
        let dir = TempDir::new().unwrap();
        write_yaml(&dir, "deployment.yaml", DEPLOYMENT_YAML);
        std::fs::write(dir.path().join("readme.txt"), "ignore me").unwrap();
        std::fs::write(dir.path().join("config.toml"), "[settings]").unwrap();

        let registry = ChecklistRegistry::load_directory(dir.path()).unwrap();
        assert_eq!(registry.list().len(), 1);
    }

    #[test]
    fn reload_picks_up_new_file() {
        let dir = TempDir::new().unwrap();
        write_yaml(&dir, "deployment.yaml", DEPLOYMENT_YAML);

        let mut registry = ChecklistRegistry::load_directory(dir.path()).unwrap();
        assert_eq!(registry.list().len(), 1);

        write_yaml(&dir, "security.yaml", SECURITY_YAML);
        registry.reload(dir.path()).unwrap();

        assert_eq!(registry.list().len(), 2);
        assert!(registry.get("security-checklist").is_some());
    }

    #[test]
    fn reload_removes_deleted_file() {
        let dir = TempDir::new().unwrap();
        write_yaml(&dir, "deployment.yaml", DEPLOYMENT_YAML);
        write_yaml(&dir, "security.yaml", SECURITY_YAML);

        let mut registry = ChecklistRegistry::load_directory(dir.path()).unwrap();
        assert_eq!(registry.list().len(), 2);

        std::fs::remove_file(dir.path().join("security.yaml")).unwrap();
        registry.reload(dir.path()).unwrap();

        assert_eq!(registry.list().len(), 1);
        assert!(registry.get("security-checklist").is_none());
    }

    #[test]
    fn load_directory_error_on_nonexistent_path() {
        let result = ChecklistRegistry::load_directory(Path::new("/nonexistent/path/xyz"));
        assert!(result.is_err());
    }

    #[test]
    fn load_directory_error_on_malformed_yaml() {
        let dir = TempDir::new().unwrap();
        write_yaml(&dir, "bad.yaml", "{ not: valid: yaml: [[[");

        let result = ChecklistRegistry::load_directory(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn info_severity_parses_correctly() {
        let dir = TempDir::new().unwrap();
        write_yaml(&dir, "info.yaml", r#"
name: info-checklist
version: "1.0.0"
mode: do-confirm
items:
  - id: info-step
    title: Informational step
    command: "echo OK"
    expected: "OK"
    severity: info
    depends_on: []
"#);
        let registry = ChecklistRegistry::load_directory(dir.path()).unwrap();
        let cl = registry.get("info-checklist").unwrap();
        assert!(matches!(cl.items[0].severity, CheckSeverity::Info));
    }
}
