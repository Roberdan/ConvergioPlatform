# ConvergioPlatform

Convergio unified control plane: mesh daemon, dashboard, and evolution engine.

## Evolution Engine

Evolution Engine v3 turns Convergio into a hypothesis-driven self-optimisation loop: telemetry collection, multi-domain evaluation, proposal generation, controlled experiments, and automated guardrails. It also integrates AutoPilot views, ROI tracking, and non-functional validation for release confidence. See architecture details in [`docs/evolution/architecture.md`](docs/evolution/architecture.md).

## Architecture

| Layer | Path | Purpose |
|-------|------|---------|
| Daemon | `daemon/` | Rust: mesh networking, TUI, HTTP/WS/SSE API |
| Dashboard | `dashboard/` | Control Room web app (Python + vanilla JS + Maranello DS) |
| Evolution | `evolution/` | Self-improving optimization engine (core + adapters) |
| Scripts | `scripts/` | Platform tooling: mesh ops, hooks, utilities |
| Docs | `docs/` | ADRs, architecture, guides |

## Quick Start

```bash
# Dashboard
cd dashboard && python3 server.py

# Daemon
cd daemon && cargo run --release

# Evolution Engine (after scaffold)
cd evolution && npm run dev
```

## Relationship to Other Convergio Repos

| Repo | Role | Relationship |
|------|------|-------------|
| `convergio` | Backend platform | Production services |
| `MaranelloLuceDesign` | Design system | UI components, canary repo |
| `ConvergioCLI` | Native CLI | C++ client |
| `convergio.io` | Gateway/frontend | User-facing web |
| **This repo** | Control plane | Orchestration, monitoring, optimization |

## License

MPL-2.0 — © Roberdan 2026

## Status (v0.1.0)

| Component | Files | Status |
|-----------|-------|--------|
| Dashboard | 496 | Migrated from ~/.claude |
| Daemon (Rust) | 107 .rs | Migrated + mesh merged |
| Evolution Engine | 7 .ts | Scaffold with real types |
| Scripts | 16 .sh | Migrated |
| **Total** | **333** | Plan 664 complete |
