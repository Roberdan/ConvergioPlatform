// API fetch functions → PopupContent for drill-down overlays.
// Split across detail_plan, detail_mesh, detail_workspace to stay under 250 lines each.

mod detail_mesh;
mod detail_plan;
mod detail_workspace;

pub use detail_mesh::{fetch_agent_detail, fetch_node_detail, parse_node_detail};
pub use detail_plan::{fetch_plan_detail, fetch_task_detail, parse_plan_detail};
pub use detail_workspace::{
    fetch_deliverable_detail, fetch_workspace_detail, format_event_detail,
};
