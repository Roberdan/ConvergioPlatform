# ADR-0111: TUI Extension — 9 Views with WebSocket Real-Time

## Status: Accepted
## Date: 23 Marzo 2026

## Context
The Convergio TUI (daemon/src/tui/) had 4 views in a single-file architecture. Plan G extends it to 9 views with real-time WebSocket updates and remote daemon support.

## Decision
- Split monolithic files into sub-modules (views/, widgets/, api/) to stay under 250-line limit
- Add 5 new views: Brain Canvas, Cost Center, Events Stream, Workspace, Deliverables
- WebSocket client with exponential backoff + HTTP fallback
- --api-url flag for remote daemon connection
- Maranello palette in ANSI 256-color
- ratatui 0.30.0 (canvas widget not available — brain view uses unicode art)

## Consequences
- 20 Rust files in daemon/src/tui/ (from 5)
- 68+ tests across inline modules
- All files under 250 lines
- Tab navigation 1-9 for all views
