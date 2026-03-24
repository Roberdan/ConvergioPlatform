<!-- Copyright (c) 2026 Roberto D'Angelo. MPL-2.0. -->
# Convergio Platform

Unified agentic AI control plane — Rust daemon, dashboard, native `CommandCenter`, and evolution engine for orchestrating AI agents across any model, tool, and machine.

> Not affiliated with or endorsed by Microsoft Corporation.

---

## What is Convergio

Convergio Platform is a self-improving AI orchestration system. You describe a goal; Ali (Chief of Staff, Opus) assembles a team of specialized agents, coordinates them across models and machines, validates output through domain-specific validators, and delivers structured results with cost, duration, and learnings.

```bash
convergio solve "Build a SaaS MVP for fitness tracking"
```

Ali handles everything: domain analysis, talent selection (89 agents, 119 skills), plan creation, agent dispatch, real-time monitoring, validation, and knowledge capture.

Website: [convergio.io](https://convergio.io) (coming soon)

---

## Quick Start

```bash
git clone https://github.com/Roberdan/ConvergioPlatform.git
cd ConvergioPlatform
./setup.sh                         # env vars, CLI aliases, enable overlay
cd daemon && cargo build --release # build Rust daemon
convergio daemon install           # auto-start on boot
convergio solve "your goal"        # Ali assembles a team and solves
```

The `cvg` CLI (short alias) provides direct access to all daemon operations:

```bash
cvg plan list                              # list active plans
cvg task update 8792 done "Summary"        # mark task done
cvg wave merge 685 2075                    # merge wave to main
cvg checkpoint save 685                    # save plan state
cvg kb search "error keywords"             # search knowledge base
cvg agent list                             # list active agents
cvg audit --path .                         # audit project for violations
cvg skill lint path/to/skill.yaml          # validate skill definition
cvg workspace list                         # list active workspaces
cvg workspace create --plan 698 --wave 1   # create workspace for wave
cvg workspace create-feature my-branch     # create feature workspace
```

Common options:

```bash
convergio solve "problem" --autonomous      # no approval gates
convergio solve "problem" --approve-each    # approve every step
convergio solve "problem" --context doc.pdf # attach document context
convergio pause [run_id]                    # suspend, preserve state
convergio resume [run_id]                  # resume paused run
cvg run list                                # list all execution runs
convergio stop [run_id]                     # abort execution
```

### CommandCenter (native macOS app)

`CommandCenter/` is the native operator surface for ConvergioPlatform. It replaces the old
web-embedded mission control path with first-class SwiftUI views for plans, agents, mesh,
evolution, run costs, PTY terminal access, the realtime brain graph, and native
notifications.

---

## Architecture

| Layer | Path | Lang | Purpose |
|---|---|---|---|
| **Daemon** | `daemon/` | Rust | IPC, mesh P2P, HTTP/WS/SSE API, SQLite WAL + CRDT, TUI, `cvg` CLI (130+ modules) |
| **Workspace** | `daemon/src/workspace/` | Rust | Agent workspace isolation, git abstraction, Release Agent, quality gates, event log |
| **Dashboard** | `dashboard/` | JS | Control Room on Maranello Luce Design — plans, mesh, chat, brain, approvals |
| **CommandCenter** | `CommandCenter/` | SwiftUI | Native macOS app — onboarding, plans, agents, mesh, evolution, costs, terminal, brain, notifications |
| **Evolution** | `evolution/` | TS | Self-improvement: telemetry → proposals → experiments |
| **Scripts** | `scripts/` | Bash | Mesh ops, platform tooling, document ingestion |
| **Config** | `claude-config/` | MD | 89 agents, 8 commands, 8 rules, 27 validation gates |

Daemon stack: axum · rusqlite WAL · tokio · ssh2 · ratatui · serde · hmac+sha2+aes-gcm · reqwest · tracing

---

## Installation

### Prerequisites

- Rust (stable toolchain, `rustup` recommended)
- Node.js 20+
- Tailscale (for mesh networking)

### Build

```bash
cd daemon && cargo build --release   # build daemon
cd daemon && cargo test              # run daemon tests (900+ tests)
cd CommandCenter && ruby Scripts/generate_xcodeproj.rb \
  && DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
     xcodebuild -project CommandCenter.xcodeproj -scheme CommandCenter -destination 'platform=macOS' build
cd evolution && npx tsc --noEmit     # type-check evolution
cd evolution && npx vitest run       # run evolution tests (43 tests)
cd dashboard && ./start.sh           # serve dashboard at :8420
```

### Optional ingestion tools

```bash
brew install poppler pandoc tesseract && pip install trafilatura openpyxl
```

---

## Agents

See [AGENTS.md](AGENTS.md) for the full catalog — 89 agents across 12 domains.

| Domain | Count | Examples |
|---|---|---|
| Core Utility | 19 | Ali, Thor, planner, reviewer, optimizer |
| Technical Dev | 11 | task-executor, Rex reviewer, Dario debugger |
| Business Ops | 11 | Davide PM, Oliver PM, Andrea customer success |
| Specialized | 14 | Omri data scientist, Fiona analyst, Ava analytics |
| Leadership | 7 | Amy CFO, Antonio strategy, Satya board |
| Compliance | 5 | Elena legal, Luca security, Dr. Enzo healthcare |

All agents support: Claude Code, Copilot CLI, OpenCode, local LLMs.

---

## OpenClaw Bridge

Convergio agents are accessible via 30+ messaging platforms (WhatsApp, Telegram, Slack, Discord, etc.) through the [OpenClaw](https://openclaw.ai) bridge.

```bash
# Generate SKILL.md files for OpenClaw
bash scripts/platform/convergio-openclaw-skills.sh

# Daemon exposes bridge API
curl http://localhost:8420/api/openclaw/agents    # list agents
curl -X POST http://localhost:8420/api/openclaw/invoke \
  -H 'Content-Type: application/json' \
  -d '{"message": "Review my code for security issues"}'
```

All requests route to Ali orchestrator, who dispatches to the right specialist. See [`integrations/openclaw-bridge/`](integrations/openclaw-bridge/) for the TypeScript plugin source.

---

## Workspace Layer

Git is invisible to agents. The daemon manages workspaces internally — agents edit files normally, hooks register operations in the event log, and the Release Agent automates the git export pipeline.

```bash
cvg workspace create --plan 698 --wave 1   # daemon creates isolated worktree
# ... agents work via Read/Edit/Write tools ...
cvg workspace release ws-1234              # quality gate → commit → PR → merge
```

| Component | What |
|---|---|
| Workspace API | `/api/workspace/create`, `/delete`, `/list`, `/status`, `/events`, `/quality-gate`, `/release` |
| GitConnector | Trait abstraction — GitHub impl via reqwest. Supports GitHub/GitLab/Gitea. |
| Release Agent | Rust module: quality gate pass → auto-commit → auto-push → auto-PR → auto-merge |
| Event Log | `workspace_events` table — CRDT-enabled audit trail independent of git history |
| Quality Gate | Mechanical checks in Rust: clean tree, file sizes, cargo check/test |

---

## Governance

Convergio Platform is governed by a formal constitution and agentic manifesto:

- [CONSTITUTION.md](CONSTITUTION.md) — operating principles, decision rights, escalation paths
- [AgenticManifesto.md](AgenticManifesto.md) — design philosophy for autonomous AI systems

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines, code standards, and the plan-driven development workflow.

---

## Legal

See [LEGAL_NOTICE.md](LEGAL_NOTICE.md) for full legal terms.

Convergio Platform is not affiliated with or endorsed by Microsoft Corporation or any other third party. All trademarks belong to their respective owners.

---

## License

[MPL-2.0](LICENSE) — Mozilla Public License 2.0

---

## Copyright

Copyright (c) 2026 Roberto D'Angelo
