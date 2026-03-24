# Task: Add Project Hierarchy View to TUI

## Context

The Convergio TUI (`daemon/src/tui/`) is a ratatui terminal dashboard with 10 tab views. The daemon API at `GET /api/project/convergio/tree` returns a hierarchical JSON with master plans and their children:

```json
{
  "project_name": "convergio",
  "total_tasks": 831,
  "done_tasks": 413,
  "plans": [
    {
      "id": 711, "name": "Convergio Vision — Business OS",
      "status": "draft", "tasks_done": 0, "tasks_total": 0,
      "is_master": true, "execution_mode": "mixed",
      "children": [
        {"id": 719, "name": "Plan H0 — TUI Hierarchy", "status": "draft", "tasks_done": 0, "tasks_total": 8, "depends_on": null},
        {"id": 712, "name": "Plan H — Hardening NASA", "status": "draft", "tasks_done": 0, "tasks_total": 7, "depends_on": "719"},
        ...
      ]
    },
    {"id": 123, "name": "Old Plan", "status": "done", ...}  // orphan (no parent)
  ]
}
```

## What to implement

### 1. Replace the Kanban view with a Project Tree view

The current PlanKanban view shows flat plan cards. Replace it with a **hierarchical project tree** that:

- Fetches from `/api/project/convergio/tree` (hardcode "convergio" for now, we'll add project switcher later)
- Shows master plans as expandable sections with `+`/`-` toggle
- Shows children indented under masters with status indicators
- Shows orphan plans (no parent) in a separate "Other Plans" section at the bottom
- Shows for each plan: name, status (color-coded), tasks progress bar, depends_on

### 2. Data model changes

In `daemon/src/tui/data.rs`, add:

```rust
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
```

Add `pub project_tree: ProjectTreeData` to `TuiData`.

### 3. API fetch

In `daemon/src/tui/api/` (check existing files for pattern), add a function:

```rust
pub async fn fetch_project_tree(api_url: &str) -> Option<ProjectTreeData> {
    let url = format!("{api_url}/api/project/convergio/tree");
    // fetch, parse JSON, map to ProjectTreeData
}
```

Follow the same pattern as existing fetch functions in that directory.

### 4. View rendering

Create `daemon/src/tui/views/project_tree.rs`:

- Render master plans as sections with colored header
- `+` prefix for masters, `-` prefix for children
- Status colors: done=green, doing=yellow, draft=muted, blocked=red, cancelled=dark gray
- Progress bar for each plan: `[████░░░░] 7/10`
- depends_on shown as `← H` (abbreviation)
- execution_mode shown as badge `[mixed]` `[sequential]` `[parallel]`
- Master rollup: show aggregate progress from children
- Selected item highlighted, Enter to drill into plan detail (existing drill_down)

### 5. Wire into tab system

In `views/mod.rs`, replace `PlanKanban` with the new tree view call, or add as a new view. Keep the tab key `1`.

### 6. Refresh

Add project_tree fetch to the refresh cycle (existing `refresh.rs` pattern). Refresh every 10s.

## Files to read first

```
daemon/src/tui/data.rs          — all data models
daemon/src/tui/views/kanban.rs  — current plan kanban (replace or augment)
daemon/src/tui/views/mod.rs     — view routing, tab bar
daemon/src/tui/api/             — API fetch functions (follow pattern)
daemon/src/tui/refresh.rs       — data refresh logic
daemon/src/tui/app.rs           — main app state, keyboard handling
daemon/src/tui/drill_down.rs    — existing drill-down
daemon/src/tui/widgets/         — shared widgets (progress bar, colors)
```

## Constraints

- Max 250 lines per file
- Use existing color palette from `widgets/` (ACCENT, OK, WARN, MUTED, etc.)
- Tests for: data model parsing, tree rendering with empty data, tree with master+children
- `cargo check` must pass
- Do NOT modify the API endpoint — it works correctly
- Do NOT remove existing views — the tree replaces or augments PlanKanban (tab 1)
