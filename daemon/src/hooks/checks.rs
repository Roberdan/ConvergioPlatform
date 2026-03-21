use rusqlite::{Connection, OpenFlags};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::PathBuf;

use super::checks_helpers::{check_warn_bash_antipatterns, check_worktree_guard, contains_any};

#[derive(Debug)]
pub struct HookCommand {
    pub tool_name: String,
    pub command: String,
}

#[derive(Debug, Default)]
pub struct DispatchState {
    pub gh_token: Option<String>,
    pub notices: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CheckOutcome {
    Continue,
    Deny(String),
    Block(String),
}

#[derive(Debug)]
pub struct CheckContext {
    pub home_dir: PathBuf,
    pub cwd: PathBuf,
    pub repo_root: Option<PathBuf>,
    pub current_branch: Option<String>,
    pub allow_main_write: bool,
    pub gh_tokens: BTreeMap<String, String>,
    pub now_epoch: i64,
    pub db_path: PathBuf,
    pub preflight_dir: PathBuf,
    pub active_plan_id: Option<u64>,
    db_conn: RefCell<Option<Connection>>,
    db_open_count: Cell<usize>,
}

impl CheckContext {
    pub fn from_env(home: &str) -> Self {
        let home_dir = PathBuf::from(home);
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let branch = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Self {
            db_path: home_dir.join(".claude/data/dashboard.db"),
            preflight_dir: home_dir.join(".claude/data/execution-preflight"),
            home_dir,
            cwd,
            repo_root: None,
            current_branch: branch,
            allow_main_write: false,
            gh_tokens: BTreeMap::new(),
            now_epoch: now,
            active_plan_id: None,
            db_conn: RefCell::new(None),
            db_open_count: Cell::new(0),
        }
    }

    pub fn for_tests() -> Self {
        Self {
            home_dir: PathBuf::from("/tmp"),
            cwd: PathBuf::from("/tmp"),
            repo_root: None,
            current_branch: None,
            allow_main_write: false,
            gh_tokens: BTreeMap::new(),
            now_epoch: 1_800_000_000,
            db_path: PathBuf::from("/tmp/dashboard.db"),
            preflight_dir: PathBuf::from("/tmp"),
            active_plan_id: None,
            db_conn: RefCell::new(None),
            db_open_count: Cell::new(0),
        }
    }

    pub fn with_db<T, F>(&self, op: F) -> Result<Option<T>, String>
    where
        F: FnOnce(&Connection) -> rusqlite::Result<T>,
    {
        if !self.db_path.exists() {
            return Ok(None);
        }
        let mut slot = self.db_conn.borrow_mut();
        if slot.is_none() {
            let conn = Connection::open_with_flags(&self.db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|err| err.to_string())?;
            self.db_open_count.set(self.db_open_count.get() + 1);
            *slot = Some(conn);
        }
        let result = op(slot.as_ref().expect("db connection")).map_err(|err| err.to_string())?;
        Ok(Some(result))
    }

    pub fn db_open_count(&self) -> usize {
        self.db_open_count.get()
    }
}

pub type CheckFn =
    fn(&HookCommand, &CheckContext, &mut DispatchState) -> Result<CheckOutcome, String>;

pub fn bash_checks() -> [CheckFn; 7] {
    [
        super::checks_support::check_gh_auto_token,
        check_worktree_guard,
        check_warn_bash_antipatterns,
        super::checks_support::check_prefer_ci_summary,
        super::checks_support::check_warn_infra_plan_drift,
        super::checks_support::check_enforce_execution_preflight,
        check_plan_db_validation_hints,
    ]
}

pub fn check_plan_db_validation_hints(
    command: &HookCommand,
    _context: &CheckContext,
    state: &mut DispatchState,
) -> Result<CheckOutcome, String> {
    if command.command.contains("plan-db.sh update-task")
        && contains_any(&command.command, &[" done", " submitted"])
    {
        state.notices.push("Hint: plan-db.sh enforces done/submitted transitions. Use plan-db-safe.sh update-task <id> done ...".to_string());
    }
    if command.command.contains("plan-db.sh start") {
        state.notices.push(
            "Hint: plan-db.sh start already enforces planner gates via cmd_check_readiness."
                .to_string(),
        );
    }
    if command.command.contains("plan-db.sh complete") {
        state
            .notices
            .push("Hint: plan-db.sh complete already enforces Thor completion gates.".to_string());
    }
    Ok(CheckOutcome::Continue)
}
