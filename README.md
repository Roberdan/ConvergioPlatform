# ConvergioPlatform

Unified AI agent swarm: Rust daemon, mesh networking, plan-driven development, centralized telemetry, self-improving evolution engine. Model-agnostic, tool-agnostic, machine-agnostic.

## What is Convergio?

A **virtual organization** you can rent. Give it a problem — it assembles a team of specialized AI agents, coordinates them, and delivers the result. Like hiring an entire agency, but the staff are AI models running on any provider.

- **84+ specialized agents**: engineers, designers, PMs, QA, strategists, compliance experts, DevOps
- **Any model**: Claude, GPT, Gemini, local LLMs (oMLX/LiteLLM)
- **Any tool**: Claude Code, Copilot CLI, OpenCode, raw API
- **Any machine**: local, Tailscale mesh nodes, cloud
- **One command**: `convergio solve "build me a fitness tracking MVP"`

Ali (Chief of Staff, Opus) analyzes your problem, picks the right specialists from the catalog, creates an execution plan, dispatches agents, monitors progress through Thor quality gates, and reports results with evidence.

## Architecture

| Layer | Path | Language | Purpose |
|---|---|---|---|
| **Daemon** | `daemon/` | Rust | P2P mesh, HTTP/WS/SSE API, IPC engine, SQLite WAL + CRDT, TUI |
| **Dashboard** | `dashboard/` | JS | Control Room on [MaranelloLuceDesign](https://github.com/Roberdan/MaranelloLuceDesign) |
| **Evolution** | `evolution/` | TypeScript | Self-improvement: telemetry → proposals → experiments → apply |
| **Config** | `claude-config/` | Markdown | Canonical agent/skill/rule definitions |
| **Scripts** | `scripts/` | Bash | Mesh ops, plan DB, agent bridge, digests |

## Quick Start

```bash
git clone https://github.com/Roberdan/ConvergioPlatform.git
cd ConvergioPlatform
./setup.sh                              # env vars, verify symlinks
cd daemon && cargo build --release      # build Rust daemon
./daemon/start.sh                       # start daemon on :8420
convergioOn                             # enable swarm overlay
convergio list                          # see available agents
convergio menu                          # interactive agent launcher
```

## Virtual Organization

```bash
convergio solve "Build a SaaS MVP for fitness tracking with React + Rust backend"
```

Ali (Chief of Staff) will:
1. Analyze → domains: frontend, backend, UX, QA, DevOps
2. Query catalog → find `sara-ux-ui-designer`, `baccio-tech-architect`, `task-executor`, `thor`, `marco-devops-engineer`
3. Create plan → waves with tasks, models, effort estimates
4. Dispatch → spawn agents on optimal tool/model/machine
5. Monitor → track via IPC, re-allocate on failure
6. Report → structured results with evidence and learnings

The agent catalog covers 11 domains:

| Domain | Agents | Examples |
|---|---|---|
| Core Utility | 19 | Thor, Ali, planner, reviewer, optimizer |
| Technical Dev | 11 | task-executor, Rex, Dario debugger, Baccio architect |
| Business Ops | 11 | Davide PM, Oliver PM, Andrea customer success |
| Specialized | 14 | Omri data scientist, Fiona analyst, Ava analytics |
| Leadership | 7 | Amy CFO, Antonio strategy, Satya board |
| Compliance | 5 | Elena legal, Luca security, Dr. Enzo healthcare |
| Release Mgmt | 5 | app-release-manager, feature-release, ecosystem-sync |
| Design/UX | 3 | Jony creative director, Sara UX/UI |
| Research | 1 | research-report-generator |
| Reference | 5 | Playbooks (Dario, Otto) |

## Convergio CLI

```bash
# Solve (virtual organization)
convergio solve "your problem here"    # Ali assembles team and solves it

# Agents
convergio list                          # available agents
convergio menu                          # interactive selection
convergio planner                       # launch as planner (Claude)
convergio executor pippo --tool copilot # launch as "pippo" on Copilot
convergio as mario --tool opencode      # custom name on OpenCode

# Communication (via daemon IPC)
convergio who                           # active agents across all machines
convergio msg pippo pluto "review auth" # send directed message
convergio read pluto                    # read messages
convergio broadcast pippo "standup"     # message all agents

# Toggle
convergio on                            # enable in ALL repos
convergio off                           # clean Claude/Copilot
convergio status                        # current state + daemon health

# Workflow
convergio plans                         # active plans
convergio session                       # git + plans + PRs status
convergio learnings                     # review session learnings
convergio auto-update                   # analyze + propose improvements

# Telemetry
convergio models                        # available models (cloud + local)
convergio metrics                       # system metrics
convergio skills                        # agent skill pool
convergio alerts                        # pending notifications
```

## Agent Swarm

13 named agents, each with a specific role and default model:

| Agent | Role | Model | Tool |
|---|---|---|---|
| `planner` | Plan creation + orchestration | opus | claude |
| `executor` | Task execution with TDD | codex | copilot |
| `thor` | Quality validation (10 gates) | opus | claude |
| `prompt` | Requirements extraction | opus | claude |
| `reviewer` | Independent plan review | sonnet | claude |
| `rex` | Code review — patterns + security | haiku | claude |
| `debugger` | Adversarial 3-hypothesis debugging | sonnet | claude |
| `postmortem` | Plan execution analysis | opus | claude |
| `optimizer` | Context + token optimization | opus | claude |
| `auditor` | Deep cross-validated repo audit | opus | claude |
| `convergio` | Platform control plane expert | sonnet | claude |
| `llm` | Local LLM infrastructure | sonnet | claude |
| `check` | Quick session status recap | mini | copilot |

Agents communicate through the daemon IPC engine — works cross-terminal, cross-machine, cross-tool.

## Plan-Driven Development

```
/prompt → /planner → review → DB → /execute → Thor → merge → learnings
```

| Phase | Agent | What |
|---|---|---|
| Requirements | `prompt` | Extract F-xx requirements from user input |
| Planning | `planner` | Create spec.yaml with waves, tasks, models |
| Review | `reviewer` | Independent quality review (fresh context) |
| Execution | `executor` | TDD, one task at a time, drift detection |
| Validation | `thor` | 10-gate quality check per wave (zero tolerance) |
| Merge | — | Squash merge into main via worktree |
| Learning | `postmortem` | Extract learnings → knowledge base |
| Calibration | — | Auto-calibrate estimation accuracy |

## Daemon API (76+ endpoints)

| Category | Examples |
|---|---|
| Plans | `/api/plan-db/list`, `/api/plan-db/execution-tree/:id` |
| IPC | `/api/ipc/agents`, `/api/ipc/send`, `/api/ipc/messages` |
| Mesh | `/api/mesh`, `/api/mesh/topology`, `/api/mesh/metrics` |
| Evolution | `/api/evolution/proposals`, `/api/evolution/roi` |
| Dashboard | `/api/overview`, `/api/tokens/daily` |
| Real-time | `WS /ws/dashboard`, `WS /ws/brain`, `WS /ws/pty` |
| Telemetry | `/api/ipc/metrics`, `/api/ipc/models`, `/api/ipc/budget` |

## Telemetry & Learning

| System | Storage | Updates |
|---|---|---|
| Knowledge Base | `knowledge_base` table | `plan-db.sh kb-write` |
| Plan Learnings | `plan_learnings` table | After task/wave/plan |
| Plan Actuals | `plan_actuals` table | Tokens, cost, ROI, Thor rejections |
| Metrics History | `metrics_history` table | Runtime, mesh, agent performance |
| Earned Skills | `earned_skills` table | Promoted learnings → reusable skills |
| Evolution | `evolution_proposals` table | Hypothesis → experiment → apply/reject |
| Session Signals | `session-learnings.jsonl` | Real-time execution signals |

## Mesh Network

Tailscale P2P mesh with HMAC-SHA256. One coordinator + N workers.

```bash
convergio heartbeat                     # check all nodes
scripts/mesh/mesh-provision-node.sh     # provision new node
scripts/mesh/mesh-sync-all.sh           # sync config across mesh
```

## Testing

```bash
cd daemon && cargo test                 # Rust daemon (107 modules)
cd evolution && npx vitest run          # Evolution engine (43 tests)
convergio status                        # verify CLI + daemon + symlinks
```

## Ecosystem

| Repo | Role |
|---|---|
| **ConvergioPlatform** (this) | Control plane, swarm orchestration |
| [MaranelloLuceDesign](https://github.com/Roberdan/MaranelloLuceDesign) | Ferrari Luce design system |
| convergio | Backend platform (Go + Python) |
| ConvergioCLI | Native CLI (C++) |
| convergio.io | Gateway / frontend |

## License

[MPL-2.0](LICENSE) — Mozilla Public License 2.0
