# ADR-0115: TUI Project Hierarchy (Plan 719)

**Status**: Accepted
**Date**: 26 Marzo 2026
**Plan**: 719 (H0 — TUI Project Hierarchy)

## Context

The TUI needed to support the new Project → Master Plan → Plans → Waves → Tasks hierarchy introduced in the Vision Master v2.0 roadmap. The existing flat plan list could not convey parent-child relationships, dependency chains, or delegation status.

## Decision

W1: Added ProjectView tab, master plan tree with expand/collapse, hierarchy context bar on drill-down (showing parent + siblings), and rollup progress bars with aggregate percentages. W2: Added execution mode badges (SEQ/PAR/MIX/CND) with semantic colors, delegation status under mesh nodes, ASCII dependency graph for master children, and Ctrl+P project switcher with session persistence.

## Consequences

8 new TUI features across 2 waves. 279 TUI tests (from ~217 baseline). New modules: `hierarchy_bar`, `dep_graph`, `project_switcher`, `persistence`. Tree navigation extends `InteractiveState` with `hierarchy_context`, `expanded_masters`, and project switcher fields. Delegation display cross-references agent/mesh data.
