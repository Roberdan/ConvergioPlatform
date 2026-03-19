# ADR-0101: Dashboard Rebuild with Maranello Presentation Runtime

**Date:** 2026-03-19
**Status:** Accepted

## Context

The Control Room dashboard had grown to ~14K LOC of vanilla JavaScript with 30+ CSS files, hardcoded colors, and no component architecture. Maintaining and extending it was increasingly difficult.

## Decision

Rebuild the dashboard using MaranelloLuceDesign v4.17.0 Presentation Runtime (AppShellController, DashboardRenderer, ViewRegistry, PanelOrchestrator, NavigationModel, StateScaffold) and Web Components. No build step — vanilla JS with Maranello IIFE bundle served by the existing Rust daemon.

## Consequences

- Consistent Ferrari Luce design across all views
- 4-theme support with zero custom CSS for colors
- WCAG 2.2 AA accessibility built-in
- Each view is an independent ES module (max 250 lines)
- Brain visualization preserved as custom canvas (not replaceable by WC)
