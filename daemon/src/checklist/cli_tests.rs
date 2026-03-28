// CLI handler tests — TDD RED phase before implementation.
// Why: Validates handler logic without spawning real processes.
#[cfg(test)]
mod tests {
    use crate::checklist::cli::{ChecklistCliHandler, ChecklistSummary};
    use crate::checklist::engine::{CheckItem, CheckMode, CheckSeverity, Checklist};
    use crate::checklist::registry::ChecklistRegistry;

    fn make_checklist(name: &str, mode: CheckMode) -> Checklist {
        Checklist {
            id: name.to_string(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            mode,
            items: vec![
                CheckItem {
                    id: "item-1".to_string(),
                    title: "Echo check".to_string(),
                    command: "echo ok".to_string(),
                    expected: "ok".to_string(),
                    severity: CheckSeverity::Info,
                    depends_on: vec![],
                },
                CheckItem {
                    id: "item-2".to_string(),
                    title: "True check".to_string(),
                    command: "true".to_string(),
                    expected: String::new(),
                    severity: CheckSeverity::Warning,
                    depends_on: vec![],
                },
            ],
        }
    }

    fn registry_with(checklists: Vec<Checklist>) -> ChecklistRegistry {
        ChecklistRegistry::from_checklists(checklists)
    }

    #[test]
    fn list_returns_summary_for_each_checklist() {
        let reg = registry_with(vec![
            make_checklist("deploy", CheckMode::DoConfirm),
            make_checklist("preflight", CheckMode::ReadDo),
        ]);
        let handler = ChecklistCliHandler::new();
        let summaries = handler.list_checklists(&reg);
        assert_eq!(summaries.len(), 2, "one summary per checklist");
        let names: Vec<&str> = summaries.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"deploy"));
        assert!(names.contains(&"preflight"));
    }

    #[test]
    fn summary_fields_are_populated() {
        let reg = registry_with(vec![make_checklist("deploy", CheckMode::DoConfirm)]);
        let handler = ChecklistCliHandler::new();
        let summaries = handler.list_checklists(&reg);
        let s: &ChecklistSummary = &summaries[0];
        assert_eq!(s.name, "deploy");
        assert_eq!(s.version, "1.0.0");
        assert_eq!(s.item_count, 2);
    }

    #[test]
    fn run_unknown_checklist_returns_error() {
        let reg = registry_with(vec![]);
        let handler = ChecklistCliHandler::new();
        let result = handler.run_checklist("nonexistent", None, &reg);
        assert!(result.is_err(), "missing checklist must return Err");
        let msg = result.unwrap_err();
        assert!(msg.contains("nonexistent"), "error must name the checklist");
    }

    #[test]
    fn run_known_checklist_returns_report() {
        let reg = registry_with(vec![make_checklist("deploy", CheckMode::DoConfirm)]);
        let handler = ChecklistCliHandler::new();
        let result = handler.run_checklist("deploy", None, &reg);
        assert!(result.is_ok(), "known checklist must succeed");
        let report = result.unwrap();
        assert_eq!(report.checklist_id, "deploy");
        assert_eq!(report.results.len(), 2);
    }

    #[test]
    fn validate_known_checklist_returns_results() {
        let reg = registry_with(vec![make_checklist("preflight", CheckMode::ReadDo)]);
        let handler = ChecklistCliHandler::new();
        let result = handler.validate_checklist("preflight", &reg);
        assert!(result.is_ok());
        let checks = result.unwrap();
        // validate runs items and returns CheckResult per item
        assert_eq!(checks.len(), 2, "two items → two results");
    }

    #[test]
    fn validate_unknown_checklist_returns_error() {
        let reg = registry_with(vec![]);
        let handler = ChecklistCliHandler::new();
        let result = handler.validate_checklist("missing", &reg);
        assert!(result.is_err());
    }

    #[test]
    fn run_with_explicit_mode_override() {
        // Passing Some(CheckMode::ReadDo) overrides the checklist's own mode.
        let reg = registry_with(vec![make_checklist("deploy", CheckMode::DoConfirm)]);
        let handler = ChecklistCliHandler::new();
        let result = handler.run_checklist("deploy", Some(CheckMode::ReadDo), &reg);
        assert!(result.is_ok());
        let report = result.unwrap();
        assert_eq!(report.mode, CheckMode::ReadDo, "mode override must be applied");
    }

    #[test]
    fn list_empty_registry_returns_empty_vec() {
        let reg = registry_with(vec![]);
        let handler = ChecklistCliHandler::new();
        let summaries = handler.list_checklists(&reg);
        assert!(summaries.is_empty());
    }
}
