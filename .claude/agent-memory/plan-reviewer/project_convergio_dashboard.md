---
name: convergio_dashboard_rebuild
description: Context for the ConvergioPlatform dashboard rebuild plan (Maranello Presentation Runtime)
type: project
---

Dashboard rebuild replaces ~14K LOC vanilla JS + 30 CSS files with MaranelloLuceDesign v4.17.0 Presentation Runtime. Daemon on port 8420 has 76+ API endpoints (confirmed ~90 unique routes in Rust source). Brain visualization is 1474 LOC in brain-canvas.js with 5 existing companion modules (regions, effects, interact, layout, physics-bridge).

**Why:** Legacy dashboard has no component architecture; target is AppShellController + ViewRegistry + PanelOrchestrator + DashboardRenderer.

**How to apply:** When reviewing or executing tasks in this plan, verify api.js is split into sub-modules (api-core + api-ipc), confirm brain-consciousness.js and brain-organism.js fate is addressed, and check /api/plans/timeline is wired to mn-gantt in plans.js.
