use super::types::SecurityError;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Mutex;

/// Kill switch scope — who to terminate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KillScope {
    /// Single agent by ID.
    Agent(String),
    /// All agents of a type (e.g., "executor", "claude").
    Type(String),
    /// All agents globally.
    All,
}

/// Kill switch mode — how to terminate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KillMode {
    /// Save state, finish atomic op, then terminate.
    Graceful,
    /// Immediate termination — use only in emergencies.
    Emergency,
}

/// Central kill switch for agent termination.
pub struct KillSwitch {
    /// Agents currently marked for termination.
    killed: Mutex<HashSet<String>>,
    /// Global halt flag.
    global_halt: Mutex<bool>,
}

impl KillSwitch {
    pub fn new() -> Self {
        Self {
            killed: Mutex::new(HashSet::new()),
            global_halt: Mutex::new(false),
        }
    }

    /// Execute a kill command. Returns list of affected agent IDs.
    pub fn kill(
        &self,
        scope: KillScope,
        _mode: KillMode,
        active_agents: &[(String, String)], // (id, type)
    ) -> Result<Vec<String>, SecurityError> {
        let mut killed = self.killed.lock()
            .map_err(|e| SecurityError::ConfigError(format!("lock: {e}")))?;

        let affected: Vec<String> = match scope {
            KillScope::Agent(id) => {
                killed.insert(id.clone());
                vec![id]
            }
            KillScope::Type(agent_type) => {
                let matching: Vec<String> = active_agents.iter()
                    .filter(|(_, t)| t == &agent_type)
                    .map(|(id, _)| id.clone())
                    .collect();
                for id in &matching { killed.insert(id.clone()); }
                matching
            }
            KillScope::All => {
                if let Ok(mut halt) = self.global_halt.lock() {
                    *halt = true;
                }
                let all: Vec<String> = active_agents.iter().map(|(id, _)| id.clone()).collect();
                for id in &all { killed.insert(id.clone()); }
                all
            }
        };

        Ok(affected)
    }

    /// Check if an agent has been killed.
    pub fn is_killed(&self, agent_id: &str) -> bool {
        self.killed.lock().map(|k| k.contains(agent_id)).unwrap_or(false)
            || self.global_halt.lock().map(|h| *h).unwrap_or(false)
    }

    /// Clear kill status for an agent (e.g., after restart).
    pub fn revive(&self, agent_id: &str) {
        if let Ok(mut killed) = self.killed.lock() {
            killed.remove(agent_id);
        }
    }

    /// Reset global halt.
    pub fn reset_halt(&self) {
        if let Ok(mut halt) = self.global_halt.lock() {
            *halt = false;
        }
    }
}

impl Default for KillSwitch {
    fn default() -> Self { Self::new() }
}
