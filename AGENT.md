# ConvergioPlatform — Agent Reference

> Unified control plane: daemon + dashboard + evolution engine. v1.0.0
> Repo: github.com/Roberdan/ConvergioPlatform
> Expert: @Convergio (see `.github/agents/Convergio.agent.md`)

## Quick Commands

| Command | What |
|---|---|
| `cd daemon && cargo build --release` | Build Rust daemon |
| `cd daemon && cargo check` | Type check (~5s) |
| `cd dashboard && ./start.sh` | Start Control Room |
| `cd evolution && npx tsc --noEmit` | Type check Evolution Engine |
| `plan-db.sh status convergio` | Plan status |
| `plan-db.sh kanban` | Kanban board |
| `scripts/mesh/mesh-heartbeat.sh` | Check all mesh nodes |

## Structure

| Path | What | Files |
|---|---|---|
| `daemon/src/mesh/` | P2P networking | 40 .rs |
| `daemon/src/server/` | HTTP/WS/SSE API | 32 .rs |
| `daemon/src/ipc/` | Inter-process | 15 .rs |
| `daemon/src/db/` | SQLite+CRDT | 7 .rs |
| `dashboard/` | Control Room web | ~494 files |
| `evolution/core/` | Optimization types | 7 .ts |
| `evolution/adapters/` | Per-target adapters | 3 .ts |
| `scripts/mesh/` | Mesh operations | 12 .sh |
| `data/dashboard.db` | Platform DB | SQLite WAL |

## Daemon API (22 endpoint modules)

agents · chat · coordinator · dashboard · github · heartbeat · ideas · ipc · mesh · notify · peers · peers_ext · plan_db (import/lifecycle/ops/query) · plans · workers · mesh_provision · sse · ws · ws_pty

## Evolution Types

`Metric` · `Proposal` · `Experiment` · `CapabilityProfile` · `PlatformAdapter`

Adapters: claude (agent config) · maranello (canary+UI) · dashboard (perf)

Governance: PR-only, shadow → canary → human approval → production

## DB Tables

plans · waves · tasks · knowledge_base · peer_heartbeats

Env: `DASHBOARD_DB=~/GitHub/ConvergioPlatform/data/dashboard.db`

## Mesh

Tailscale P2P, HMAC-SHA256 auth. peers.conf at `~/.claude/config/peers.conf`.

Coordinator: M5 Max. Workers: added via `mesh-provision-node.sh`.

## Conventions

Max 250 lines · English · Rust fmt+clippy · JS vanilla+Maranello · TS strict · Comments WHY only
