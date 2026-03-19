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
| Agent IPC | `daemon/src/ipc/` | Named agent registration, messaging, skill routing |
| Scripts | `scripts/` | Platform tooling: mesh ops, agent bridges, hooks |
| Docs | `docs/` | ADRs, architecture, guides |

## Quick Start

```bash
git clone <repo-url> ConvergioPlatform
cd ConvergioPlatform
./setup.sh          # symlinks, hooks, env — single command
./daemon/start.sh   # start daemon (IPC on port 8420)
```

## Agent Communication (IPC)

Agents (Claude Code, Copilot) register and communicate via the daemon IPC system.

| What | Command |
|------|---------|
| Register agent | `scripts/platform/agent-bridge.sh --register --name planner --type claude` |
| Send message | `curl -X POST localhost:8420/api/ipc/send -H 'Content-Type: application/json' -d '{"sender_name":"planner","channel":"planning","content":"W2 starting"}'` |
| List agents | `curl localhost:8420/api/ipc/agents` |
| Sync skills | `scripts/platform/agent-skills-sync.sh` |

- API reference: [`docs/api/ipc-agents.md`](docs/api/ipc-agents.md)
- Agent onboarding: [`docs/guides/agent-onboarding.md`](docs/guides/agent-onboarding.md)
- Architecture decision: [`docs/adr/0002-agent-bridge-decoupling.md`](docs/adr/0002-agent-bridge-decoupling.md)

## Testing

```bash
# Agent IPC E2E tests (requires daemon running)
scripts/platform/test-agent-ipc.sh

# Setup idempotency tests
scripts/platform/test-setup-e2e.sh

# Daemon unit tests
cd daemon && cargo test
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
