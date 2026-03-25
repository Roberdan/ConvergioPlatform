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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tui_data_default_compiles_and_is_empty() {
        let data = TuiData::default();
        assert!(data.plans.is_empty());
        assert!(data.pipeline.is_empty());
        assert!(data.mesh_nodes.is_empty());
        assert!(data.agents.is_empty());
        assert!(data.brain_nodes.is_empty());
        assert!(data.events.is_empty());
        assert!(data.workspaces.is_empty());
        assert!(data.deliverables.is_empty());
        assert!(data.chat_messages.is_empty());
        assert!(data.chat_session_id.is_none());
        assert!(data.notifications.is_empty());
        assert!(data.project_tree.plans.is_empty());
        assert!(data.projects.is_empty());
    }

    #[test]
    fn chat_message_default_has_empty_fields() {
        let msg = ChatMessage::default();
        assert!(msg.role.is_empty());
        assert!(msg.content.is_empty());
        assert!(msg.timestamp.is_empty());
    }

    #[test]
    fn chat_message_role_and_content_roundtrip() {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: "hello".to_string(),
            timestamp: "2026-03-24T10:00:00Z".to_string(),
        };
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "hello");
    }

    #[test]
    fn kpi_data_default_zeroes() {
        let kpi = KpiData::default();
        assert_eq!(kpi.plans_active, 0);
        assert_eq!(kpi.agents_running, 0);
        assert_eq!(kpi.daily_tokens, 0);
        assert_eq!(kpi.daily_cost, 0.0);
        assert_eq!(kpi.mesh_online, 0);
    }

    #[test]
    fn cost_data_default_compiles_and_is_empty() {
        let cost = CostData::default();
        assert!(cost.by_model.is_empty());
        assert!(cost.by_project.is_empty());
        assert!(cost.by_date.is_empty());
        assert_eq!(cost.summary.run_count, 0);
    }
}

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
