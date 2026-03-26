// NASA rules pack tests — verify the YAML checklist loads correctly via ChecklistRegistry.
// Why: Ensures the 7 Plan H hardening rules are present with correct IDs, mode, and severities.
#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::checklist::engine::{CheckMode, CheckSeverity};
    use crate::checklist::registry::ChecklistRegistry;

    // Path to NASA rules YAML relative to the repo root.
    // Tests run from the daemon/ directory, so we go up one level.
    fn nasa_yaml_dir() -> std::path::PathBuf {
        let manifest = env!("CARGO_MANIFEST_DIR");
        Path::new(manifest).parent().unwrap().join("config/checklists")
    }

    #[test]
    fn nasa_rules_checklist_loads_from_directory() {
        let dir = nasa_yaml_dir();
        let registry =
            ChecklistRegistry::load_directory(&dir).expect("should load config/checklists/");
        let checklist = registry.get("nasa-rules").expect("nasa-rules checklist must exist");
        assert_eq!(checklist.name, "nasa-rules");
    }

    #[test]
    fn nasa_rules_version_is_set() {
        let dir = nasa_yaml_dir();
        let registry = ChecklistRegistry::load_directory(&dir).unwrap();
        let checklist = registry.get("nasa-rules").unwrap();
        assert!(!checklist.version.is_empty(), "version must be set");
    }

    #[test]
    fn nasa_rules_mode_is_read_do() {
        let dir = nasa_yaml_dir();
        let registry = ChecklistRegistry::load_directory(&dir).unwrap();
        let checklist = registry.get("nasa-rules").unwrap();
        assert_eq!(checklist.mode, CheckMode::ReadDo, "NASA rules must use READ-DO mode");
    }

    #[test]
    fn nasa_rules_has_exactly_seven_items() {
        let dir = nasa_yaml_dir();
        let registry = ChecklistRegistry::load_directory(&dir).unwrap();
        let checklist = registry.get("nasa-rules").unwrap();
        assert_eq!(checklist.items.len(), 7, "NASA rules pack must define exactly 7 items");
    }

    #[test]
    fn nasa_rules_all_required_item_ids_present() {
        let expected_ids = [
            "bounded_loops_check",
            "function_size_check",
            "error_swallowing_check",
            "narrow_state_check",
            "nesting_gate_check",
            "assertions_check",
            "side_effect_naming_check",
        ];
        let dir = nasa_yaml_dir();
        let registry = ChecklistRegistry::load_directory(&dir).unwrap();
        let checklist = registry.get("nasa-rules").unwrap();
        let item_ids: Vec<&str> = checklist.items.iter().map(|i| i.id.as_str()).collect();
        for expected in &expected_ids {
            assert!(
                item_ids.contains(expected),
                "NASA rules checklist missing item: {expected}"
            );
        }
    }

    #[test]
    fn nasa_rules_critical_items_have_correct_severity() {
        let dir = nasa_yaml_dir();
        let registry = ChecklistRegistry::load_directory(&dir).unwrap();
        let checklist = registry.get("nasa-rules").unwrap();
        let critical_ids =
            ["bounded_loops_check", "function_size_check", "error_swallowing_check"];
        for id in &critical_ids {
            let item = checklist.items.iter().find(|i| i.id == *id).unwrap_or_else(|| {
                panic!("item {id} not found")
            });
            assert_eq!(
                item.severity,
                CheckSeverity::Critical,
                "item {id} must be critical severity"
            );
        }
    }

    #[test]
    fn nasa_rules_warning_items_have_correct_severity() {
        let dir = nasa_yaml_dir();
        let registry = ChecklistRegistry::load_directory(&dir).unwrap();
        let checklist = registry.get("nasa-rules").unwrap();
        let warning_ids = ["narrow_state_check", "nesting_gate_check", "assertions_check"];
        for id in &warning_ids {
            let item = checklist.items.iter().find(|i| i.id == *id).unwrap_or_else(|| {
                panic!("item {id} not found")
            });
            assert_eq!(
                item.severity,
                CheckSeverity::Warning,
                "item {id} must be warning severity"
            );
        }
    }

    #[test]
    fn nasa_rules_side_effect_naming_is_info_severity() {
        let dir = nasa_yaml_dir();
        let registry = ChecklistRegistry::load_directory(&dir).unwrap();
        let checklist = registry.get("nasa-rules").unwrap();
        let item = checklist
            .items
            .iter()
            .find(|i| i.id == "side_effect_naming_check")
            .expect("side_effect_naming_check must exist");
        assert_eq!(item.severity, CheckSeverity::Info);
    }

    #[test]
    fn nasa_rules_all_items_have_non_empty_commands() {
        let dir = nasa_yaml_dir();
        let registry = ChecklistRegistry::load_directory(&dir).unwrap();
        let checklist = registry.get("nasa-rules").unwrap();
        for item in &checklist.items {
            assert!(
                !item.command.is_empty(),
                "item {} must have a non-empty command",
                item.id
            );
        }
    }
}
