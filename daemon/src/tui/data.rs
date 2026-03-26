// Data structs for the TUI — all view models live here.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanCard {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub tasks_done: i64,
    pub tasks_total: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskPipelineItem {
    pub task_id: String,
    pub title: String,
    pub status: String,
    pub agent: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshNode {
    pub name: String,
    pub online: bool,
    pub role: String,
    pub cpu_percent: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentOrgNode {
    pub name: String,
    pub role: String,
    pub host: String,
    pub active_task: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct KpiData {
    pub plans_active: i64,
    pub agents_running: i64,
    pub daily_tokens: i64,
    pub daily_cost: f64,
    pub mesh_online: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrainNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub parent_id: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CostEntry {
    pub model: String,
    pub calls: i64,
    pub cost_usd: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CostByDate {
    pub date: String,
    pub cost_usd: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CostSummary {
    pub run_count: i64,
    pub avg_duration_secs: f64,
    pub total_cost_usd: f64,
}

// All fields default (Vec::new() + CostSummary::default())
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CostData {
    pub by_model: Vec<CostEntry>,
    pub by_project: Vec<CostEntry>,
    pub by_date: Vec<CostByDate>,
    pub summary: CostSummary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceEvent {
    pub id: i64,
    pub workspace_id: String,
    pub agent: String,
    pub action: String,
    pub file_path: Option<String>,
    pub detail: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceInfo {
    pub workspace_id: String,
    pub path: String,
    pub branch: String,
    pub plan_id: Option<i64>,
    pub status: String,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeliverableInfo {
    pub id: i64,
    pub name: String,
    pub output_type: String,
    pub status: String,
    pub version: i64,
    pub project_id: String,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    pub id: i64,
    pub title: String,
    pub message: String,
    pub severity: String,
    pub read: bool,
    pub created_at: String,
}

// --- Chat data ---

// --- Delegation data: which plan each mesh peer is running ---

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DelegationInfo {
    pub peer_name: String,
    pub plan_name: String,
    pub plan_id: i64,
    pub tasks_done: i64,
    pub tasks_total: i64,
    pub agent_name: String,
    pub last_heartbeat: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: String,    // "user" or "assistant"
    pub content: String,
    pub timestamp: String,
}

// --- Project list data ---

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectInfo {
    pub id: i64,
    pub name: String,
    pub path: String,
}

// --- Plan Hierarchy Context (shown as context bar when drilling into a sub-plan) ---

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SiblingPlanInfo {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub tasks_done: i64,
    pub tasks_total: i64,
    pub is_current: bool,
    pub depends_on: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlanHierarchyContext {
    pub master_name: String,
    pub master_id: i64,
    pub siblings: Vec<SiblingPlanInfo>,
}


// --- Project Tree data ---

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProjectTreeNode {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub tasks_done: i64,
    pub tasks_total: i64,
    pub is_master: bool,
    pub depends_on: Option<String>,
    pub execution_mode: Option<String>,
    pub children: Vec<ProjectTreeNode>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProjectTreeData {
    pub project_name: String,
    pub total_tasks: i64,
    pub done_tasks: i64,
    pub plans: Vec<ProjectTreeNode>,
}

// --- Aggregate view model ---

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TuiData {
    pub plans: Vec<PlanCard>,
    pub pipeline: Vec<TaskPipelineItem>,
    pub mesh_nodes: Vec<MeshNode>,
    pub agents: Vec<AgentOrgNode>,
    pub kpis: KpiData,
    pub brain_nodes: Vec<BrainNode>,
    pub cost: CostData,
    pub events: Vec<WorkspaceEvent>,
    pub workspaces: Vec<WorkspaceInfo>,
    pub deliverables: Vec<DeliverableInfo>,
    pub chat_messages: Vec<ChatMessage>,
    pub chat_session_id: Option<String>,
    pub notifications: Vec<Notification>,
    pub project_tree: ProjectTreeData,
    pub projects: Vec<ProjectInfo>,
    pub delegations: Vec<DelegationInfo>,
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod tests;

// --- View selector ---

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MainView {
    #[default]
    PlanKanban,
    TaskPipeline,
    MeshStatus,
    AgentOrgChart,
    BrainCanvas,
    CostCenter,
    EventStream,
    WorkspaceView,
    Deliverables,
    Chat,
    ProjectView,
}
