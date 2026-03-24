// DrillDownRequest — identifies what async fetch to perform on Enter.
// Set synchronously by handle_enter; resolved by process_post_key.

/// Identifies what drill-down to perform when Enter is pressed.
/// Set by handle_enter (sync); resolved by process_post_key (async).
#[derive(Debug, Clone)]
pub enum DrillDownRequest {
    /// Drill into a plan by its DB id.
    Plan(i64),
    /// Drill into a task: (plan_id, task_index in pipeline list).
    Task(i64, usize),
    /// Drill into a mesh node by peer name.
    MeshNode(String),
    /// Drill into an agent by agent_id.
    Agent(String),
    /// Drill into a workspace event by index in data.events.
    Event(usize),
    /// Drill into a workspace by workspace_id.
    Workspace(String),
    /// Drill into a deliverable by DB id.
    Deliverable(i64),
}
