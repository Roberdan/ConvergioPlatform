<!-- Copyright (c) 2026 Roberto D'Angelo. MPL-2.0. -->
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
| **Ingestion** | `scripts/platform/` | Bash | Document ingestion: PDF/DOCX/XLSX/URL/folder → markdown |
| **Config** | `claude-config/` | MD | 89 agents, 8 commands, 8 rules, 27 validation gates |
| **Scripts** | `scripts/` | Bash | 10 CLI scripts, mesh ops, plan DB, digests |

## Quick Start

```bash
git clone https://github.com/Roberdan/ConvergioPlatform.git
cd ConvergioPlatform
./setup.sh                         # env vars, CLI aliases, enable overlay
cd daemon && cargo build --release # build Rust daemon
convergio daemon install           # auto-start on boot
convergio daemon menubar           # status icon in menu bar (◉/◎)
convergio solve "your goal"        # Ali takes over
```

## Convergio CLI

```bash
convergio solve "problem"               # Ali assembles team and solves
convergio solve "problem" --autonomous   # no approval needed
convergio solve "problem" --approve-each # approve every step
convergio solve "problem" --context doc.pdf  # attach document context
convergio stop [run_id]                  # abort execution
convergio pause [run_id]                 # suspend, preserve state
convergio resume [run_id]                # resume paused run
convergio metrics run <id>               # per-run cost, duration, agents
convergio metrics runs                   # list all execution runs

convergio planner                        # launch as planner (Claude Opus)
convergio executor pippo --tool copilot  # "pippo" on Copilot (GPT Codex)
convergio list                           # all 89 agents

convergio who                            # active agents (all machines)
convergio msg pippo pluto "review auth"  # direct message
convergio watch-agents                   # real-time activity stream

convergio plans                          # active plans
convergio autopilot watch                # auto execute→Thor→merge loop
convergio learnings analyze              # find recurring patterns
convergio org stats                      # organizational statistics

convergio on / convergio off             # enable/disable overlay
convergio status                         # overlay + daemon state
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

## Document Ingestion

Feed external knowledge into any execution run with `--context`:

```bash
convergio solve "Analyze this contract" --context contract.pdf
convergio solve "Summarize report" --context https://example.com/report
convergio solve "Review specs" --context ./specs-folder/
```

`convergio-ingest.sh` converts any source to structured markdown before the run starts:

| Format | Tool required | Fallback |
|---|---|---|
| PDF | `pdftotext` (poppler) | warning, skip |
| DOCX / PPTX | `pandoc` | warning, skip |
| XLSX / CSV | `python3` + `openpyxl` | warning, skip |
| URL | `trafilatura` | `curl` + basic strip |
| Images | `tesseract` | warning, skip |
| Markdown / text | — | always works |

Install all tools: `brew install poppler pandoc tesseract && pip install trafilatura openpyxl`

## Approvals Dashboard

The **Approvals** view in the Control Room lets operators review pending plans before execution proceeds:

| Action | What it does |
|---|---|
| Approve | Marks plan active, execution continues |
| Cancel | Aborts plan, notifies all agents |
| Pause | Suspends execution, preserves state |

Access: `http://localhost:8420` → sidebar → Approvals (only plans with `approval_required=true` appear).

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
| Autonomy | `--autonomous` / `--approve-plan` / `--approve-each` |
| Budget cap | `$CONVERGIO_MAX_BUDGET` (default $10/day) |
| Abort | `convergio stop [run_id]` — broadcast ABORT |
| Toggle | `convergioOff` / `convergioOn` |

## Daemon API (82+ endpoints)

IPC: `agents`, `send`, `messages`, `channels`, `context`, `locks` | Plans: `list`, `execution-tree`, `validate-wave` | Mesh: `peers`, `topology`, `delegate` | Evolution: `proposals`, `approve`, `experiments` | Runs: `GET /api/runs`, `GET /api/runs/:id`, `POST /api/runs/:id/pause`, `POST /api/runs/:id/resume` | Metrics: `GET /api/metrics` | Ingestion: `POST /api/ingest` | Real-time: `WS /ws/dashboard`, `SSE /api/chat/stream`

CLI scripts (`convergio-run-ops.sh`, `convergio-metrics.sh`, `convergio-ingest.sh`) are thin wrappers over these endpoints; they fall back to read-only `sqlite3` with a warning when the daemon is not running.

## Mesh Network

Tailscale P2P, HMAC-SHA256, CRDT DB sync. `convergio heartbeat` to check all nodes.

## Testing

```bash
cd daemon && cargo test                 # Rust daemon (478 tests, 140 modules)
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
