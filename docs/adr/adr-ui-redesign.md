# ADR: UI Redesign — 3-Layer Architecture

**Date:** 2026-03-19 | **Status:** Accepted | **Plan:** #671

## Context

ConvergioPlatform needed a unified native UI combining terminal productivity, system tray access, and rich web visualization (brain canvas, evolution charts, Maranello DS components).

## Decision

Adopt a 3-layer architecture: **TUI** (ratatui) + **Menu Bar** (SwiftUI) + **Dashboard** (WKWebView).

| Layer | Tech | Purpose |
|---|---|---|
| TUI | ratatui | Plan/task/agent ops, keyboard-driven |
| Menu Bar | SwiftUI + AppKit | System tray, quick actions, status |
| Dashboard | WKWebView | Brain viz, evolution, chat, KPI |

**WKWebView over pure SwiftUI** — brain canvas uses D3/canvas rendering and Maranello DS web components. Rebuilding in SwiftUI would duplicate effort with no gain. WKWebView reuses existing dashboard JS and enables hot-reload during development.

**Brain strip pattern** — compact horizontal bar showing active agents, tasks, sessions. Tapping expands to full brain canvas. Driven by WS `/ws/brain` push events (`agent_update`, `task_update`, `session_update`).

**Drawer pattern** — slide-over panels for detail views (plan detail, agent inspector, evolution proposals). Avoids full navigation; maintains context. Menu bar triggers drawers via IPC.

**Evolution section** — dedicated dashboard tab with proposal list, approve/reject actions, experiment timeline, ROI metrics. All data via `/api/evolution/*` REST endpoints. Audit trail per proposal.

## Consequences

- Three codebases to maintain (Rust TUI, Swift menu bar, JS dashboard)
- WKWebView requires macOS 11+; acceptable for target audience
- Brain strip + drawer patterns keep UI responsive without deep navigation hierarchies
