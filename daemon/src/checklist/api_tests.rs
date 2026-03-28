// API handler tests — TDD RED phase.
// Why: Validates request/response shapes and RunStore semantics without HTTP stack.
#[cfg(test)]
mod tests {
    use crate::checklist::api::{
        ChecklistApiState, ChecklistRunRequest, ChecklistRunResponse, RunStore,
    };
    use crate::checklist::engine::{CheckItem, CheckMode, CheckSeverity, Checklist};
    use crate::checklist::registry::ChecklistRegistry;


    fn make_checklist(name: &str) -> Checklist {
        Checklist {
            id: name.to_string(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            mode: CheckMode::DoConfirm,
            items: vec![CheckItem {
                id: "step-1".to_string(),
                title: "Echo test".to_string(),
                command: "echo hello".to_string(),
                expected: "hello".to_string(),
                severity: CheckSeverity::Info,
                depends_on: vec![],
            }],
        }
    }

    fn make_state(checklists: Vec<Checklist>) -> ChecklistApiState {
        let registry = ChecklistRegistry::from_checklists(checklists);
        ChecklistApiState::new(registry)
    }

    #[test]
    fn run_store_stores_and_retrieves_report() {
        let store = RunStore::new();
        let _checklist = make_checklist("deploy");
        // Build a minimal report
        use crate::checklist::engine::{CheckResult, CheckStatus, ExecutionReport};
        use chrono::Utc;
        use std::time::Duration;
        let result = CheckResult {
            item_id: "step-1".to_string(),
            status: CheckStatus::Pass,
            message: "ok".to_string(),
            timestamp: Utc::now(),
        };
        let report = ExecutionReport::from_results(
            "deploy".to_string(),
            CheckMode::DoConfirm,
            vec![result],
            Duration::from_millis(10),
        );
        let run_id = "run-001".to_string();
        store.insert(run_id.clone(), report.clone());
        let retrieved = store.get(&run_id);
        assert!(retrieved.is_some(), "stored report must be retrievable");
        assert_eq!(retrieved.unwrap().checklist_id, "deploy");
    }

    #[test]
    fn run_store_returns_none_for_unknown_id() {
        let store = RunStore::new();
        assert!(store.get("does-not-exist").is_none());
    }

    #[test]
    fn api_state_run_executes_known_checklist() {
        let state = make_state(vec![make_checklist("preflight")]);
        let req = ChecklistRunRequest { name: "preflight".to_string(), mode: None };
        let result = state.handle_run(req);
        assert!(result.is_ok(), "known checklist run must succeed");
        let resp: ChecklistRunResponse = result.unwrap();
        assert!(!resp.run_id.is_empty(), "run_id must be non-empty");
        assert_eq!(resp.report.checklist_id, "preflight");
    }

    #[test]
    fn api_state_run_unknown_checklist_returns_error() {
        let state = make_state(vec![]);
        let req = ChecklistRunRequest { name: "ghost".to_string(), mode: None };
        let result = state.handle_run(req);
        assert!(result.is_err());
    }

    #[test]
    fn api_state_status_returns_stored_report() {
        let state = make_state(vec![make_checklist("preflight")]);
        let req = ChecklistRunRequest { name: "preflight".to_string(), mode: None };
        let resp = state.handle_run(req).unwrap();
        let run_id = resp.run_id;
        let status = state.handle_status(&run_id);
        assert!(status.is_some(), "status must be retrievable by run_id");
        assert_eq!(status.unwrap().checklist_id, "preflight");
    }

    #[test]
    fn api_state_status_returns_none_for_missing_run() {
        let state = make_state(vec![]);
        assert!(state.handle_status("no-such-run").is_none());
    }

    #[test]
    fn api_state_list_returns_all_names() {
        let state = make_state(vec![make_checklist("alpha"), make_checklist("beta")]);
        let names = state.handle_list();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"beta".to_string()));
    }

    #[test]
    fn run_request_mode_override_applied() {
        let state = make_state(vec![make_checklist("deploy")]);
        let req = ChecklistRunRequest {
            name: "deploy".to_string(),
            mode: Some("read-do".to_string()),
        };
        let result = state.handle_run(req);
        assert!(result.is_ok());
        let resp = result.unwrap();
        // ReadDo runner is used
        use crate::checklist::engine::CheckMode;
        assert_eq!(resp.report.mode, CheckMode::ReadDo);
    }

    #[test]
    fn run_request_invalid_mode_returns_error() {
        let state = make_state(vec![make_checklist("deploy")]);
        let req = ChecklistRunRequest {
            name: "deploy".to_string(),
            mode: Some("warp-speed".to_string()),
        };
        let result = state.handle_run(req);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("warp-speed"), "error must name the invalid mode");
    }
}
