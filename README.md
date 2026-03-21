# ConvergioPlatform

A **virtual organization** powered by AI. Give it a problem — it assembles a team of specialized agents, coordinates them across any model/tool/machine, and delivers the result. Like renting an entire agency.

## How It Works

```bash
convergio solve "Build a SaaS MVP for fitness tracking"
```

Ali (Chief of Staff, Opus) does everything:
1. **Analyzes** the problem — identifies domains: frontend, backend, UX, QA, DevOps
2. **Queries the talent pool** — 89 agents, 119 skills, confidence-weighted selection
3. **Creates a plan** — waves, tasks, per-task model/tool/validator assignment
4. **Dispatches agents** — spawns them on optimal tool (Claude/Copilot/OpenCode/local LLM)
5. **Monitors** — real-time via daemon IPC, re-dispatches on failure, budget cap enforcement
6. **Validates** — domain-specific validators (Thor for code, doc-validator for docs, compliance-validator for legal)
7. **Reports** — structured results with cost, duration, learnings

Agents communicate like Slack — channels, DMs, broadcasts. Messages arrive automatically via push notifications. No polling needed.

## Architecture

```
User → convergio solve → Ali (orchestrator)
                           ├→ spawns agents (any model, any tool, any machine)
                           ├→ coordinates via daemon IPC (channels, DMs, broadcast)
                           ├→ validates via domain validators (Thor, doc, strategy, design, compliance)
                           └→ learns (knowledge base, skills, calibration)
                                    ↕
                              Daemon :8420 (Rust)
                           ├→ IPC engine (messaging, registry, shared context, file locks)
                           ├→ Mesh P2P (Tailscale, HMAC-SHA256, CRDT sync)
                           ├→ Plan DB (SQLite WAL, waves, tasks, validation triggers)
                           ├→ Evolution (proposals → experiments → apply)
                           └→ Telemetry (metrics, budget, cost tracking)
```

| Layer | Path | Lang | Purpose |
|---|---|---|---|
| **Daemon** | `daemon/` | Rust | IPC, mesh P2P, HTTP/WS/SSE API, SQLite WAL + CRDT, TUI |
| **Dashboard** | `dashboard/` | JS | Control Room on [MaranelloLuceDesign](https://github.com/Roberdan/MaranelloLuceDesign) |
| **Evolution** | `evolution/` | TS | Self-improvement: telemetry → proposals → experiments |
| **Config** | `claude-config/` | MD | 89 agents, 8 commands, 8 rules, 27 validation gates |
| **Scripts** | `scripts/` | Bash | 10 CLI scripts, mesh ops, plan DB, digests |

## Quick Start

```bash
git clone https://github.com/Roberdan/ConvergioPlatform.git
cd ConvergioPlatform
./setup.sh                    # env vars, CLI aliases, enable overlay
cd daemon && cargo build --release
./daemon/start.sh             # daemon on :8420
convergio list                # see 89 agents
convergio solve "your goal"   # Ali takes over
```

Disable anytime: `convergioOff`. Re-enable: `convergioOn`. Full revert: `revert-claude-symlinks.sh --env`.

## Convergio CLI

```bash
# Virtual Organization
convergio solve "problem"               # Ali assembles team and solves
convergio solve "problem" --autonomous   # no approval needed
convergio solve "problem" --approve-each # approve every step
convergio stop [run_id]                  # abort execution

# Named Agents (any model, any tool)
convergio planner                        # launch as planner (Claude Opus)
convergio executor pippo --tool copilot  # "pippo" on Copilot (GPT Codex)
convergio as mario --tool opencode       # custom name on OpenCode
convergio as amy --tool local            # local LLM via LiteLLM
convergio menu                           # interactive selection
convergio list                           # all 89 agents

# Communication (daemon IPC — like Slack for agents)
convergio who                            # active agents (all machines)
convergio msg pippo pluto "review auth"  # direct message
convergio read pluto                     # read inbox
convergio broadcast pippo "standup"      # message all
convergio watch-agents                   # real-time activity stream (WebSocket)

# Cross-Repo Coordination
convergio sync request virtualbpm maranello "Need VoiceOrb component"
convergio sync pending                   # show pending requests
convergio sync auto-dispatch             # Ali handles cross-repo requests

# Organizational Telemetry
convergio org live                       # last N agent interactions
convergio org matrix                     # who talks to whom
convergio org teams                      # active teams per run
convergio org flow <run_id>              # timeline of a run
convergio org stats                      # overall statistics

# Workflow
convergio plans                          # active plans
convergio session                        # git + plans + PRs
convergio autopilot watch                # auto execute→Thor→merge loop
convergio auto-update                    # analyze learnings, propose improvements

# Telemetry & Learning
convergio models                         # available models (cloud + local)
convergio metrics                        # system metrics
convergio skills                         # agent skill pool
convergio alerts                         # pending notifications
convergio learnings analyze              # find recurring patterns
convergio learnings promote              # auto-promote to knowledge/skills
convergio collect-metrics                # snapshot system telemetry

# Toggle & Setup
convergio on                             # enable in ALL repos
convergio off                            # clean Claude/Copilot
convergio status                         # overlay + daemon state
convergio sync-agents                    # regenerate tool-specific agent files
convergio import-agents <path>           # import agents from external repo
```

## Agent Catalog (89 agents, 12 domains)

| Domain | Count | Examples | Default Model |
|---|---|---|---|
| Core Utility | 19 | Ali, Thor, planner, reviewer, optimizer | opus/sonnet |
| Technical Dev | 11 | task-executor, Rex reviewer, Dario debugger, Baccio architect | codex/haiku |
| Business Ops | 11 | Davide PM, Oliver PM, Andrea customer success | sonnet/haiku |
| Specialized | 14 | Omri data scientist, Fiona analyst, Ava analytics | haiku |
| Leadership | 7 | Amy CFO, Antonio strategy, Satya board | sonnet |
| Compliance | 5 | Elena legal, Luca security, Dr. Enzo healthcare | opus |
| Release Mgmt | 5 | app-release-manager, ecosystem-sync | sonnet |
| Design/UX | 3 | Jony creative director, Sara UX/UI | sonnet |
| Research | 1 | research-report-generator | sonnet |
| Reference | 5 | Playbooks (Dario debugger, Otto performance) | — |

All agents support: Claude Code, Copilot CLI, OpenCode, local LLMs. Model and tool selected per-task by Ali or planner.

## Validation System (5 validators, 27 gates)

Not just code. Every output type has its own validator:

| Output Type | Validator | Gates | Examples |
|---|---|---|---|
| `pr` (code) | Thor | 10 | tests, lint, type check, scope, integration, TDD |
| `document` | doc-validator | 5 | completeness, structure, sources, coherence, actionability |
| `analysis` | strategy-validator | 4 | data quality, completeness, feasibility, alignment |
| `design` | design-validator | 4 | accessibility (WCAG), consistency, user flow, responsive |
| `legal_opinion` | compliance-validator | 4 | regulations, risk, gaps, remediation |

Enforced by DB trigger: tasks can't be marked `done` without validation by the assigned validator.

## Agent Communication

The daemon IPC engine works like Slack for agents:

| Feature | How |
|---|---|
| Channels | `ipc_channels` — auto-created, named (general, dm:pippo, planning) |
| Direct messages | `POST /api/ipc/send` with agent name |
| Broadcast | Send to channel, all subscribers see it |
| Who's online | `GET /api/ipc/agents` — name, host, type, last_seen |
| Push notifications | Notification hook checks inbox automatically — no polling |
| Shared context | `ipc_shared_context` — pass artifacts between agents (LWW) |
| File locks | `ipc_file_locks` — prevent two agents editing same file |
| Real-time stream | `WS /ws/dashboard` — live agent events |
| Cross-repo | `convergio sync` — requests/dispatch between repos |

Agents are born dynamically (spawned by Ali or user), communicate via IPC, and die when their task is complete. Ali re-spawns on failure (3 attempts with exponential backoff).

## Plan-Driven Development

```
/prompt → /planner → review → DB → /execute → validate → merge → learn
```

| Phase | Agent | Output Type | Validator |
|---|---|---|---|
| Requirements | `prompt` | — | — |
| Planning | `planner` | plan | user approval |
| Review | `reviewer` | review | — |
| Execution | per-task agent | per-task | per-task validator |
| Validation | Thor / domain validator | — | — |
| Merge | — | — | pre-merge gates |
| Learning | `postmortem` | — | — |
| Calibration | auto | — | — |

Supports non-code workstreams: research, strategy, design, legal, marketing, analysis.

## Telemetry & Learning

| System | Storage | Purpose |
|---|---|---|
| Agent Events | `ipc_messages` + `agent_events` | Who did what, when, to whom |
| Execution Runs | `execution_runs` | Per-run: goal, team, cost, duration, result |
| Knowledge Base | `knowledge_base` | Domain insights, confidence, hit count |
| Plan Learnings | `plan_learnings` | Post-mortem: estimation misses, rejections |
| Plan Actuals | `plan_actuals` | Tokens, cost, ROI, Thor rejection rate |
| Metrics History | `metrics_history` | CPU, memory, agent count, plan progress |
| Earned Skills | `earned_skills` | Promoted learnings → reusable skills |
| Agent Catalog | `agent_catalog` + `agent_skills` | 89 agents, 119 skill mappings |
| Validation Gates | `validation_gates` | 27 gates across 5 output types |
| Evolution | `evolution_proposals` | Hypothesis → experiment → apply/reject |
| Budget | `execution_runs.cost_usd` | Daily cap ($10 default), per-run tracking |

## Safety & Controls

| Control | How |
|---|---|
| Autonomy levels | `--autonomous`, `--approve-plan`, `--approve-each` |
| Budget cap | `$CONVERGIO_MAX_BUDGET` (default $10/day) — autopilot pauses on exceed |
| Secret scanner | Pre-commit hook blocks API keys, tokens, passwords, hardcoded URLs |
| Agent health | Zombie detection (10 min timeout), auto-prune |
| Retry backoff | Exponential (30s, 60s, 120s), escalate after 3 failures |
| Abort | `convergio stop [run_id]` — broadcast ABORT to all agents |
| Values | Security (OWASP), Accessibility (WCAG 2.1 AA), Responsibility (GDPR), Compliance |
| Toggle | `convergioOff` removes all overlay — instant clean state |
| Revert | `revert-claude-symlinks.sh --env` — full rollback including env vars |

## Daemon API (76+ endpoints)

| Category | Key Endpoints |
|---|---|
| IPC | `agents`, `send`, `messages`, `channels`, `context`, `locks`, `conflicts` |
| Plans | `list`, `execution-tree`, `start`, `complete`, `validate-wave` |
| Mesh | `peers`, `topology`, `metrics`, `delegate`, `heartbeat` |
| Evolution | `proposals`, `approve`, `reject`, `experiments`, `roi` |
| Dashboard | `overview`, `tokens/daily`, `notifications` |
| Real-time | `WS /ws/dashboard`, `WS /ws/brain`, `WS /ws/pty` |
| Streaming | `SSE /api/chat/stream`, `SSE /api/plan/preflight` |

## Mesh Network

Tailscale P2P mesh with HMAC-SHA256. One coordinator + N workers. CRDT-enabled DB sync.

```bash
convergio heartbeat                     # check all nodes
convergio sync register-repo vbpm ~/GitHub/VirtualBPM
convergio sync request vbpm maranello "Need component X"
```

## Testing

```bash
cd daemon && cargo test                 # Rust daemon (140 modules)
cd evolution && npx vitest run          # Evolution engine (43 tests)
convergio status                        # CLI + daemon + symlinks
convergio org stats                     # organizational telemetry
convergio learnings summary             # knowledge system health
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
