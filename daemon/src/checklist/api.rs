// Checklist REST API — axum-compatible handlers for checklist operations.
// Why: Exposes engine functionality over HTTP so dashboard and agents can
//      trigger, monitor, and retrieve checklist runs without CLI access.
use crate::checklist::engine::{CheckMode, ExecutionReport};
use crate::checklist::registry::ChecklistRegistry;
use crate::checklist::runners::do_confirm::DoConfirmRunner;
use crate::checklist::runners::read_do::ReadDoRunner;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// ── Request / Response types ──────────────────────────────────────────────────

/// Body for `POST /api/checklists/run`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistRunRequest {
    pub name: String,
    /// Optional mode override: "do-confirm" or "read-do".
    pub mode: Option<String>,
}

/// Response for a successful `POST /api/checklists/run`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistRunResponse {
    pub run_id: String,
    pub report: ExecutionReport,
}

// ── RunStore ──────────────────────────────────────────────────────────────────

/// Thread-safe in-memory store for completed execution reports.
/// Keyed by run_id (UUID string).  Used for `GET /api/checklists/status/{run_id}`
/// and `GET /api/checklists/results/{run_id}`.
#[derive(Clone)]
pub struct RunStore {
    inner: Arc<Mutex<HashMap<String, ExecutionReport>>>,
}

impl RunStore {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(HashMap::new())) }
    }

    /// Persist a report under `run_id`.
    pub fn insert(&self, run_id: String, report: ExecutionReport) {
        self.inner.lock().expect("RunStore lock").insert(run_id, report);
    }

    /// Retrieve a stored report by run_id.
    pub fn get(&self, run_id: &str) -> Option<ExecutionReport> {
        self.inner.lock().expect("RunStore lock").get(run_id).cloned()
    }
}

impl Default for RunStore {
    fn default() -> Self {
        Self::new()
    }
}

// ── ChecklistApiState ─────────────────────────────────────────────────────────

/// Shared state threaded through axum handlers via `axum::Extension`.
///
/// Owns the registry (read-only after init) and the run store (mutable).
#[derive(Clone)]
pub struct ChecklistApiState {
    registry: Arc<ChecklistRegistry>,
    store: RunStore,
}

impl ChecklistApiState {
    pub fn new(registry: ChecklistRegistry) -> Self {
        Self { registry: Arc::new(registry), store: RunStore::new() }
    }

    /// Handle `POST /api/checklists/run`.
    pub fn handle_run(&self, req: ChecklistRunRequest) -> Result<ChecklistRunResponse, String> {
        let checklist = self
            .registry
            .get(&req.name)
            .ok_or_else(|| format!("checklist '{}' not found", req.name))?;

        let mode = match req.mode.as_deref() {
            None => checklist.mode.clone(),
            Some("do-confirm") => CheckMode::DoConfirm,
            Some("read-do") => CheckMode::ReadDo,
            Some(other) => {
                return Err(format!(
                    "unknown mode '{}'; expected 'do-confirm' or 'read-do'",
                    other
                ))
            }
        };

        let mut cl = checklist.clone();
        cl.mode = mode.clone();

        let report = match mode {
            CheckMode::DoConfirm => DoConfirmRunner::new().run(&cl),
            CheckMode::ReadDo => ReadDoRunner::new().execute(&cl),
        };

        let run_id = Uuid::new_v4().to_string();
        self.store.insert(run_id.clone(), report.clone());
        Ok(ChecklistRunResponse { run_id, report })
    }

    /// Handle `GET /api/checklists/status/{run_id}`.
    pub fn handle_status(&self, run_id: &str) -> Option<ExecutionReport> {
        self.store.get(run_id)
    }

    /// Handle `GET /api/checklists/results/{run_id}` — same as status, full report.
    pub fn handle_results(&self, run_id: &str) -> Option<ExecutionReport> {
        self.store.get(run_id)
    }

    /// Handle `GET /api/checklists/list`.
    pub fn handle_list(&self) -> Vec<String> {
        let mut names: Vec<String> =
            self.registry.list().into_iter().map(|cl| cl.name.clone()).collect();
        names.sort();
        names
    }
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
