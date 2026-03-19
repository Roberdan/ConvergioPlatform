# ConvergioPlatform

Unified control plane for the Convergio ecosystem: Rust mesh daemon, real-time dashboard, evolution engine, and AI agent orchestration across a multi-node network.

## Architecture

| Layer | Path | Language | What it does |
|-------|------|----------|-------------|
| **Daemon** | `daemon/` | Rust | P2P mesh networking (Tailscale + HMAC-SHA256), HTTP/WS/SSE API, TUI, IPC engine, SQLite WAL + CRDT sync |
| **Dashboard** | `dashboard/` | JS | Control Room web app built on [MaranelloLuceDesign](https://github.com/Roberdan/MaranelloLuceDesign) Presentation Runtime |
| **Evolution** | `evolution/` | TypeScript | Self-improving optimization loop: telemetry, proposals, experiments, guardrails |
| **Scripts** | `scripts/` | Bash | Mesh operations, platform tooling, agent bridges, nightly guardians |
| **Config** | `claude-config/` | Markdown | Shared Claude Code configuration (agents, skills, rules, commands) |

## Quick Start

```bash
git clone https://github.com/Roberdan/ConvergioPlatform.git
cd ConvergioPlatform
./setup.sh                    # symlinks, hooks, env
cd daemon && cargo build --release  # build Rust daemon
./dashboard/start.sh          # Control Room on http://localhost:8420
```

## Dashboard

The Control Room is served by the Rust daemon on port 8420. Built with MaranelloLuceDesign v4.17.0:

- **8 views**: Overview, Plans, Mesh, Brain, Ideas, IPC, Admin, Terminal
- **Real-time**: WebSocket for live updates, SSE for streaming operations
- **4 themes**: Editorial, Nero, Avorio, Colorblind (WCAG 2.2 AA)
- **Components**: `mn-app-shell`, `mn-chart`, `mn-data-table`, `mn-gauge`, `mn-gantt`, `mn-modal`, `mn-tabs`
- **Brain**: Neural visualization of agent activity (force-directed canvas)

## Mesh Network

Tailscale-based P2P mesh with HMAC-SHA256 authentication. One coordinator + N worker nodes.

| Command | What |
|---------|------|
| `scripts/mesh/mesh-provision-node.sh <peer>` | Provision a new node |
| `scripts/mesh/mesh-heartbeat.sh` | Check node health |
| `scripts/mesh/mesh-sync-all.sh` | Sync config across mesh |
| `scripts/mesh/mesh-preflight.sh` | Validate tools, auth, versions |
| `scripts/platform/buongiorno.sh` | Morning routine: update all nodes |

## Agent IPC

AI agents (Claude Code, GitHub Copilot) register and communicate via the daemon IPC system.

```bash
# Register an agent
scripts/platform/agent-bridge.sh --register --name planner --type claude

# List active agents
curl localhost:8420/api/ipc/agents

# Send a message
curl -X POST localhost:8420/api/ipc/send \
  -H 'Content-Type: application/json' \
  -d '{"sender_name":"planner","channel":"planning","content":"W2 starting"}'
```

See [Agent Onboarding Guide](docs/guides/agent-onboarding.md) and [IPC API Reference](docs/api/ipc-agents.md).

## Plan Database

SQLite-backed plan/task/wave tracking with CLI tooling:

```bash
plan-db.sh status              # quick status
plan-db.sh kanban              # kanban board
plan-db.sh execution-tree 665  # tree view with statuses
pianits.sh                     # CLI plan dashboard
pianits.sh kanban              # kanban shortcut
```

## Testing

```bash
cd daemon && cargo test                    # Rust daemon tests
cd evolution && npm ci && npm test         # Evolution engine tests
scripts/platform/test-agent-ipc.sh        # Agent IPC E2E (requires daemon)
npx playwright test                        # Dashboard E2E (requires daemon)
```

## API Endpoints (76 total)

The daemon exposes REST, WebSocket, and SSE endpoints on port 8420:

| Category | Examples |
|----------|---------|
| Dashboard | `GET /api/overview`, `/api/mission`, `/api/tokens/daily` |
| Plans | `GET /api/plan-db/list`, `/api/plan-db/execution-tree/:id` |
| Mesh | `GET /api/mesh`, `/api/mesh/metrics`, `/api/mesh/topology` |
| IPC | `GET /api/ipc/agents`, `/api/ipc/budget`, `/api/ipc/models` |
| Ideas | `GET /api/ideas`, `POST /api/ideas` |
| Real-time | `WS /ws/dashboard`, `WS /ws/brain`, `WS /ws/pty` |
| Streaming | `SSE /api/chat/stream/:sid`, `/api/plan/preflight` |

## Ecosystem

| Repo | Role |
|------|------|
| **ConvergioPlatform** (this) | Control plane, mesh, orchestration |
| [MaranelloLuceDesign](https://github.com/Roberdan/MaranelloLuceDesign) | Ferrari Luce design system |
| convergio | Backend platform (Go + Python) |
| ConvergioCLI | Native CLI (C++) |
| convergio.io | Gateway / frontend |

## License

[MPL-2.0](LICENSE) — Mozilla Public License 2.0
