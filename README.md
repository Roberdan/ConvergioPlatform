<!-- Copyright (c) 2026 Roberto D'Angelo. MPL-2.0. -->
# Convergio Platform

Unified agentic AI control plane — Rust daemon, dashboard, and evolution engine for orchestrating AI agents across any model, tool, and machine.

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

Common options:

```bash
convergio solve "problem" --autonomous      # no approval gates
convergio solve "problem" --approve-each    # approve every step
convergio solve "problem" --context doc.pdf # attach document context
convergio pause [run_id]                    # suspend, preserve state
convergio resume [run_id]                  # resume paused run
convergio metrics runs                      # list all execution runs
convergio stop [run_id]                     # abort execution
```

---

## Architecture

| Layer | Path | Lang | Purpose |
|---|---|---|---|
| **Daemon** | `daemon/` | Rust | IPC, mesh P2P, HTTP/WS/SSE API, SQLite WAL + CRDT, TUI (107 modules) |
| **Dashboard** | `dashboard/` | JS | Control Room on Maranello Luce Design — plans, mesh, chat, brain, approvals |
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
cd daemon && cargo test              # run daemon tests (478 tests)
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
