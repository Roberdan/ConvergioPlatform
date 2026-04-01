# Changelog

## [Unreleased]

### Added
- `cvg ask` CLI command with agent alias resolution (~/.convergio/aliases.toml)
- `/interview` deep interview command for iterative requirements extraction (max 7 questions)
- Interview skill wrapper with skill.yaml and SKILL.md
- A2UI agent-to-UI block push protocol: POST /api/a2ui/push, GET /api/a2ui/blocks, SSE /api/a2ui/stream
- A2UI background TTL cleanup for expired blocks
- Config-driven inference fallback chains (`[inference.fallback]` in config.toml, hot-reloadable)
- Provider health check before fallback attempt (claude binary, gh auth, local LLM ping)
- Structured fallback logging: `[FALLBACK] primary/error/fallback`
- `/api/inference/status` now returns config-driven chains and max_attempts
- PARITY.md capability audit (17 capabilities verified with evidence)
- README.md competitive comparison table (vs claw-code, oh-my-codex, openclaw)
- Default agent aliases generated during `cvg setup`

## [20.7.0] - 01 Aprile 2026

### Added
- Org-plan linkage: plans table has org_id column, GET /api/orgs/:slug/plans endpoint
- Org timeline: ipc_org_events table, GET/POST /api/orgs/:slug/timeline
- Org metrics: GET /api/orgs/:slug/metrics (day/week/month aggregation), GET /api/orgs/:slug/report
- Brain API enrichment: /api/brain now returns orgs array with health/budget/agents and agent_relations
- Global orgchart: GET /api/orgs/chart — all orgs in one ASCII view
- Service marketplace: GET /api/services/marketplace, POST /api/services/request, PUT /api/services/requests/:id
- Night worker config: [night] section in config.toml, GET /api/node/night-status
- SubagentStop auto-submit hook: scripts/platform/subagent-auto-submit.sh

### Fixed
- Mesh test fixtures: added missing lan_ip field to PeerConfig/ResolvedPeer test constructors
- Evidence gate timeout: increased cargo_test timeout from 60s to 180s

## [20.6.0] - 01 Aprile 2026

### Added
- **MCP Completeness**: 22 new MCP tools (platform_tools: 12, org_tools: 10) — total 43 MCP tools
- `POST /api/plan-db/task/create` + `cvg task create` CLI for adding tasks to in-progress plans
- **3-tier enforcement**: Git hooks (5, both tools), Claude hooks (10), Daemon-side (5) — 20 total checks
- Multi-transport mesh: peer resolver probes Thunderbolt > LAN > Tailscale with 1s TCP connect
- `lan_ip` field in peers.conf for LAN-first connectivity

### Fixed
- Evidence gate uses isolated `CARGO_TARGET_DIR=/tmp/convergio-evidence-build` (was blocking daemon)
- Evidence gate timeout reduced 180s to 60s
- Copilot CLI now enforced by git hooks (pre-commit + commit-msg)
- Daemon CWD guard prevents boot from worktree
- Auth token auto-provisioned if missing
- TaskReaper resets orphan in_progress tasks from dead agents
- MainDirtyReaper notifies when main has >5 dirty files

### Removed
- M3 Max from mesh (hardware given away)
- 60+ agent definitions from `.claude/agents/` context (saved 3.8k tokens)

## [20.5.0] - 01 Aprile 2026

### Added
- **Org Factory**: `cvg org create-org` — creates virtual org from mission with CEO, departments, agents, night agents
- **Org From Repo**: `cvg org create-org-from` — scans existing repo (languages, frameworks, CI, deps) and creates matching org
- **Repo Scanner**: analyzes folder for tech stack, structure, dependencies, CI configuration
- **Night Agents**: per-org scheduled agents (daily_report, pr_monitor, issue_triage, dep_update) on Haiku for cost savings
- **Orgchart Renderer**: ASCII orgchart (full box-drawing + compact for Telegram)
- **Orgchart API**: `GET /api/orgs/:slug/orgchart` (JSON + `?format=ascii`)
- **Telegram intents**: "crea org X", "analizza repo /path" wired to org factory
- **Jarvis tools**: `create_org` + `scan_repo` for autonomous tool calling (12 tools total)

### Changed
- `CreateProject` Telegram intent now uses full org factory instead of basic org creation
- Voice router: broader keyword matching for org creation ("crea org", "crea organizzazione")

## [20.4.0] - 01 Aprile 2026

### Added
- Jarvis fallback to Claude: `AskAli` removed, unrecognized messages escalate to Claude with full platform context
- Keyword enrichment: report/analysis/action intents routed to Claude
- Mesh version tracking: `version` + `rustc_version` in peer heartbeats
- `GET /api/mesh/update-status` endpoint for version comparison across mesh
- Mesh auto-update: background task (5min interval, quiet hours, rate limit, coordinator builds, workers rsync)
- `start.sh` restart loop with automatic rollback on health failure
- `cvg setup` onboarding wizard (planned — config.toml hot-reload architecture)

### Changed
- `keyword_classify` fallback: AskAli → EscalateToAli (Claude with context)
- start.sh: single-run → restart loop with rollback support

### Fixed
- Tool count test assertion (7→10 tools after health/history/mesh additions)

## [20.3.0] - 01 Aprile 2026

### Added
- `GET /api/agents/history` — filtered agent activity with since/until/status/model/limit params
- `cvg agent history` CLI — table-formatted agent execution history (last 7 days default)
- `scripts/platform/daemon-install.sh` — installs daemon binary to `~/.convergio/bin/`
- `exit_reason` column in `agent_activity` table for post-mortem analysis
- Build isolation: worktree `cargo check` uses separate `CARGO_TARGET_DIR`
- **Jarvis intelligence**: always-on context (plans, agents, costs, health, mesh, history)
- **Multi-round tool calling**: Jarvis can call 8 platform tools autonomously (max 3 rounds)
- **Telegram conversation memory**: sliding window of 5 exchanges per chat
- **CreateProject intent**: "crea progetto X" via Telegram → org + bootstrap plan + 3 tasks
- **AskOrg intent**: "come sta X?" → org status summary with members/budget/decisions

### Changed
- Agent activity retention expanded from 1 hour to 30 days
- Copilot worker default timeout: 600s → 1200s
- Agent reaper stale threshold: 30min → 60min (avoids false reaps on long tasks)
- Context budget: 2000 → 4000 tokens (30% of subagents were exhausting budget)
- `daemon/start.sh` prefers installed binary from `~/.convergio/bin/`
- Jarvis max tokens: 512 → 2048 for complete analytical responses

### Fixed
- Daemon no longer crashes when worktree `cargo check` locks `target/` directory
- MCP agent_complete handler field name mismatch (`name` → `agent_id`)
- `exit_reason` UPDATE uses COALESCE to avoid overwriting with NULL
- ISO datetime T-separator parsing in agent history queries
- Constitution violations: split 3 files over 250 lines (cli_task, api_ipc/mod, background_sync)
- MCP tool count test assertions updated (19→31 for Ring 1, 21→33 for Ring 0)
- `cvg status` auth: uses Bearer token for protected endpoints (was showing "Cannot reach daemon")
- 13 TUI tests: `Option<Terminal>` so tests pass without TTY (CI, SSH, GitHub Actions)
- Tailscale test: dynamic HostName from `tailscale status --json` instead of hardcoded peer mapping

## [20.2.0] - 31 Marzo 2026

### Added
- **MCP Completeness (Plan 10037 W1)**: 12 new MCP tools exposing pre-existing daemon APIs (`cvg_create_plan`, `cvg_start_plan`, `cvg_create_task`, `cvg_record_validation`, `cvg_health_deep`, `cvg_list_workspaces`, `cvg_remember`, `cvg_recall`, `cvg_budget`, `cvg_agent_catalog`, `cvg_quality_gate`, `cvg_list_messages`) in `platform_tools.rs`
- `POST /api/plan-db/task/create` endpoint + `cvg task create` CLI — add tasks to plans in any status
- **Workflow hardening** (incident 2026-03-31): MainGuard hook, git pre-commit hook, DaemonCwdGuard, auth token auto-provision, TaskReaper for orphan tasks, MainDirtyReaper for dirty repo detection

### Fixed
- Daemon no longer starts from worktree CWD (was causing evidence gate failures)
- `CONVERGIO_AUTH_TOKEN` auto-provisioned if missing (was causing 401 on all API calls after restart)
- Orphan `in_progress` tasks from crashed copilots auto-reset to `pending` by reaper
- Removed agent symlinks from `.claude/agents/` (was loading 3.8k tokens of 60+ agents into context)

### Security
- MainGuard: blocks Edit/Write to `daemon/src/` on main branch (Claude Code hook)
- git pre-commit: blocks commits on main in main checkout (works for copilot too)
- MainDirtyReaper: daemon notifies via Telegram/ntfy if main has >5 dirty files

## [20.1.2] - 31 Marzo 2026

### Added
- Agent Network integration coverage: end-to-end org flow test (`daemon/src/server/api_orgs/integration_tests.rs`) validating org creation, bootstrap steps, intra/inter-org messaging, decision log, telemetry, SSE stream endpoint, org detail payload, and budget gate blocking.
- CLI org chart command: `cvg bus org` renders terminal hierarchy tree from `/api/orgs`.
- Architecture decision record: `docs/adr/adr-agent-network.md` documenting the network-of-companies model and operational tradeoffs.

### Changed
- `ws_brain_org` tests now use unique DB paths per run to prevent cross-test UNIQUE constraint collisions.
- Troubleshooting and command references updated for org APIs and `cvg org`/`cvg bus` operations.

## [20.1.1] - 31 Marzo 2026

### Fixed
- Validation queue/verdict endpoints now auto-create tables on fresh nodes (was 500)
- Sync replication: disable FK checks during import (plans referencing absent projects)
- Documentation: TROUBLESHOOTING.md updated with v20.1 sync, gates, delegation, provisioning
- CLAUDE.md: added Task Lifecycle section, updated build commands to include all features

### Verified
- 4/4 consecutive e2e runs: create plan → sync → cross-node update → reverse sync
- Fresh node provisioning: clone, rsync binary, start, all endpoints 200
- Concurrent updates: LWW resolves, conflicts logged (_sync_conflicts)
- Kill + recovery: daemon restarts, sync resumes within 30s
- TestGate blocks submitted without evidence (400)
- ValidatorGate blocks done without verdict (400)

## [20.1.0] - 30 Marzo 2026

### Added
- Hard enforcement gates: TestGate (evidence before submitted), ValidatorGate (Thor verdict before done)
- Task evidence API: POST/GET /api/plan-db/task/evidence for recording test/build/lint passes
- Validation record API: POST /api/validation/record for Thor verdict shortcut
- Full delegation pipeline: `cvg delegation start` automates rsync, worktree, plan sync, daemon restart, launch
- Background validator loop processing validation queue every 30s
- Background health probes: CLI + HTTP checks every 60s with shared state
- Convergence verification: SHA-256 checksum comparison after each sync round with drift warnings
- Audit trail middleware: automatic logging of POST/PUT/DELETE mutations to audit_log
- Sync conflict logging: LWW conflicts recorded to _sync_conflicts table
- Branch creation blocker hook: only detached worktrees allowed
- 5 new enforcement hooks: EvidenceGate, TestGate, CommitLint, FailLoud, AgentIdentity
- Scope management rules in CLAUDE.md (max 5 checkpoints per session)
- Plan-level done gate preventing redundant execution of completed plans

### Fixed
- Re-enabled HTTP LWW sync (was commented out; CRDT path broken without crsqlite)
- Fixed execute-plan.sh JSON parsing (.plan.name instead of .name)
- Fixed crsqlite feature: added rusqlite/load_extension dependency
- Fixed tui_integration test: ephemeral port instead of hardcoded 8420
- Fixed sandbox enforcement: validate_command() now called before delegation
- Fixed health probe: local CLI check instead of hitting external API
- Removed silent error swallowing in delegation pipeline and conflict logging
- Fixed all compiler warnings across kernel + crsqlite feature combinations

### Changed
- README: corrected false claims about CRDT vector clocks, sandbox enforcement, audit trail
- Template adapter canary stub now emits explicit warning
- hard-enforcement.md updated to reflect actual gate status (BLOCK vs WARNING)
- Deleted dead code: libsql_adapter_task_sync.rs, daemon_sync_auth.rs

### Security
- Auth headers added to HTTP sync and delegation pipeline API calls
- ValidatorGate prevents unauthorized status=done transitions

## [20.0.0] - 2 Aprile 2026

### Breaking Changes
- **LiteLLM removed**: zero Python dependencies; `Provider::LiteLLM` variant eliminated from the provider enum. Migrate to `ClaudeSubscription`, `CopilotSubscription`, or `LocalLLM`.

### Added
- CLI subscription providers: `ClaudeSubscription`, `CopilotSubscription`, `LocalLLM` replacing LiteLLM proxy
- Budget tracking integrated into chat pipeline (per-session and cumulative spend)
- Session IPC endpoints: `agents/list`, `agents/deregister`, `agents/send-direct`
- Timestamp-based LWW sync with conflict logging to _sync_conflicts table
- Rollback snapshots: daemon persists pre-apply snapshots for plan/task rollback
- Agent sandboxing: per-agent command validation in delegation pipeline
- Thor validator service: dedicated long-running quality-gate daemon
- Nightly autonomy job: scheduled risk-based goal evaluation (cron via daemon scheduler)
- Risk-based autonomous policy: decisions gated by configurable risk score thresholds
- Goal decomposer: hierarchical goal-to-task decomposition engine
- Mesh convergence heuristic: SHA-256 checksum comparison with drift detection
- Node self-provisioning: mesh nodes bootstrap configuration on first contact
- Approval UX: interactive approval flow for high-risk autonomous actions
- Playwright e2e test suite covering core dashboard flows

### Security
- Audit trail: mutation requests (POST/PUT/DELETE) logged with agent identity
- `--dangerously-skip-permissions` flag removed; permission checks are now non-bypassable
- Per-worktree `settings.json` isolation; settings no longer bleed across worktrees

### Fixed
- Plan counter off-by-one bug causing duplicate plan IDs under concurrent creation
- Pre-existing test imports breaking cargo test on clean checkout
- Memory endpoint path corrected from `/api/memory` to `/api/memory/list`

## [19.5.0] - 29 Marzo 2026

### Added
- Memory management API endpoints: list, stats, gc, delete (`/api/memory/*`) (Plan 758 W1)

### Changed
- Ali orchestrator: compressed to 10-domain routing with domain-specific validation (Plan 758 W2)

### Fixed
- SQLite ALTER TABLE: replaced function defaults with trigger-based updated_at (Plan 758 W1)

## [19.4.0] - 29 Marzo 2026

### Changed
- Consolidated 104 agent definitions to 69 (-34%), merged 10 duplicates, compressed prompts (Plan 757 W1)
- Consolidated 13 rule files to 2: hard-enforcement.md (~10 rules) + best-practices.md (Plan 757 W2)
- Optimized CLAUDE.md from 163 to 110 lines (-32%) (Plan 757 W2)
- Evidence gate cargo_test timeout increased to 180s (was 90s)

### Removed
- 35 redundant/duplicate agent definitions (Plan 757 W1)
- 11 rule files replaced by tiered system (Plan 757 W2)

## [19.3.0] - 29 Marzo 2026

### Changed
- Fail-loud policy: eliminated 413 of 446 silent `.ok()` and `let _ =` error-swallowing patterns across daemon (Plan 756 W1)
- Notification system returns actual per-channel delivery status, not hardcoded success (Plan 756 W2)
- Consolidated 3 CLAUDE.md copies to single source of truth at repo root (Plan 756 W3)

### Added
- Automated Rust module wiring check (`scripts/check-rust-wiring.sh`) with PostToolUse hook (Plan 756 W3)
- ADR-0124: Fail-loud policy documenting the architectural decision

### Removed
- `claude-config/agent-catalog/` — 85 duplicate agent files (126K tokens recovered) (Plan 756 W2)
- 9 reference-only files moved from `agents/` to `docs/reference/agent-protocols/` (Plan 756 W2)
- osascript fallback from notification handlers (Plan 756 W2)

## [19.2.0] - 29 Marzo 2026

### Added
- Background sync fix: DB now syncs between mesh nodes via HTTP (Plan 749 W1)
- Auto-rebuild script + launchd plist: daemon rebuilds after git pull on both nodes (Plan 749 W1)
- Delegation E2E test script: end-to-end validation of plan delegation workflow (Plan 749 W2)
- Remote sync E2E tests integrated into test-e2e.sh (Plan 749 W2)

### Fixed
- background_sync peer URLs had double http:// scheme (Plan 749 W1)
- Remote sync export SSH command URL quoting (Plan 749 W3)

## [19.1.0] - 29 Marzo 2026

### Added
- Voice engine: full pipeline — cpal audio capture, webrtc-vad, whisper-rs STT, wake word detection, intent classification (Plan 748)
- Voice engine: Voxtral 4B TTS via mlx-audio (mlx-community/Voxtral-4B-TTS-2603-mlx-4bit), Italian/English voices
- Voice engine: feature-gated (`--features voice`), opt-in with zero default build impact
- Jarvis identity: kernel renamed to Jarvis across log prefixes, Telegram responses, and /api/kernel/status
- Telegram poll health check: detect and report dead poll task
- Peer failure tracker: 3-strike consecutive failure alerts for remote nodes
- Ali escalation: unknown problems create micro-plans and launch copilot-plan-runner
- Deterministic problem triage: auto-fix daemon crash, DB lock, stale worktrees, high FD count

## [19.0.0] - 2026-03-29

### Breaking
- libSQL migration: timestamp-based sync adapter replaces crsqlite CRDT (W1)
- Data migration required: new sync columns on plans/tasks/waves tables

### Added
- HTTP sync endpoints with crsqlite gated and background_sync wired (W1)
- GET /api/plan-db/execution-context/:plan_id — complete prompt generation for delegated tasks
- Centralized peer resolver with 3-stage fuzzy match (W2, B6/B9)
- Evidence gate hardening: mutex, SHA cache, shutdown reaper (W2)
- Unified delegation workflow with cvg tool integration (W3)
- Terminal stack replication + worktree auto-cleanup (W3)
- Per-repo gh credential routing for multi-org setups (W3)
- invoke_agent MCP tool — tool 18 in MCP server (W4)
- Developer experience improvements: cheatsheet, commands, api, template CLI commands (W4)
- API telemetry instrumentation: 90 handlers, 16 tests (W5)
- IPC + CLI integration tests: 34 tests, ~80% IPC coverage (W5)
- Comprehensive mesh module tests: 310 tests, ~90% coverage (W5)
- API integration tests for previously untested endpoints (W5)
- ADR-0121 libSQL migration decision
- ADR-0122 recursive session continuity
- PreCompact hook: checkpoints plan + spawns copilot continuation

### Fixed
- Bug fixes B1-B4, B7-B8: assorted daemon reliability issues (W2)
- branch_name read from DB instead of derived from worktree path
- Plan runner: never dies on error, only resets in_progress tasks (not submitted)
- Execution-context prompt is directive (ACT IMMEDIATELY, no questions)

### Changed
- copilot-plan-runner uses daemon execution-context API instead of local prompt generation

## [18.5.0] - 2026-03-28

### Removed
- gui/ directory (legacy SwiftUI, archived)
- dashboard/ directory (replaced by convergio-web)
- convergio-daemon standalone repo (archived on GitHub)
- convergio-app repo (archived on GitHub)
- Plan R (731) cancelled (replaced by convergio-web)

### Changed
- Library name: claude_core → convergio_core

### Added
- convergio-mcp-server binary (14 tools, ring security, stdio transport)
- .mcp.json for Claude Code MCP integration
- ADR-0120 cross-repo cleanup decisions
- convergio-capabilities.md reference document

## [18.4.0] - 2026-03-27

### Added
- Siri integration via Shortcuts (scripts/siri/)
- Kernel MCP tools: 7 functions for intelligent data retrieval (plans, costs, nodes, agents)
- ChatML function calling: Mistral uses <tool_call> to query daemon API
- EscalateToAli: explicit escalation to Opus via Telegram ("ali", "opus", "cloud")
- crsqlite installed on M1 Pro (CRDT sync capability)
- Role-based node provisioning checks in mesh-provision-node.sh
- POST /api/kernel/active-node for audio routing
- Telegram voice: ffmpeg path resolution for launchd

### Fixed
- Hardcoded paths in sync-db plist replaced with placeholders
- EscalateToAli keyword priority (checked before stato/costi)

### Changed
- route_ask_ali uses /api/kernel/ask with MCP tools (not hardcoded keywords)
- Fork reconciliation: ConvergioPlatform/daemon/ is sole source of truth (ADR-0118)

## [18.3.2] - 2026-03-27

### Fixed
- mlx_lm uses space-separated subcommand (fixes deprecation warning)
- /api/kernel/ask returns human answers (not classify format)
- "stato" uses plan-db/list for accurate plan count
- Python venv auto-detected (~/convergio-env)

### Added
- GET /api/node/readiness — 10-check node health report
- scripts/mesh/deploy-node.sh — single-command node deploy
- scripts/kernel/sync-db.sh — safe DB rsync with integrity check
- Node readiness check in kernel monitor loop (every 5min)

## [18.3.1] - 27 Marzo 2026

### Added
- Plan 729 (Q) Convergio Kernel: always-on local LLM kernel watchdog on M1 Pro, Mistral 3 8B via MLX (20-30% faster than Ollama on Apple Silicon), deterministic recovery (LLM classifies, rules act), Telegram Bot API notifications (OGG voice native), audio mesh routing to active user node, macOS say TTS fallback, ADR-0116

## [18.3.0] - 27 Marzo 2026

### Added
- Plan 715 (K) Agentic Memory: SQLite+FTS5 store, vector embeddings with cosine similarity, hybrid search, blob store, Markdown export, recovery chain (Markdown→SQLite→VectorStore), REST API + CLI (cvg memory remember/recall/forget/share/attest/export/reindex), 93 tests
- Plan 721 (F2) SwiftUI Command Center: kanban board, agent catalog with live sessions, mesh topology, embedded terminal (PTY via WS), chat with Ali, menu bar indicator with notifications, 30 accessibility labels
- Plan 714 (J) MCP Client: Capability trait with 4-ring security model (Core/Trusted/Community/Sandboxed), MCP JSON-RPC stdio connector, YAML capability registry, proxy with rate limiting + circuit breaking, per-agent permissions (deny-by-default), security gate for registration, REST API + CLI (cvg capability list/invoke/register/permissions), Stripe demo, 43 tests
- Plan 718 (N) Voice: VAD (energy-based, 50ms onset), wake word detection, Whisper ASR engine (local/API), intent extraction (Command/Query/Control/Navigation), TTS via macOS say, full pipeline state machine, CLI (cvg voice start/stop/status), 18 tests
- Plan 716 (L) Security Perimeters: per-agent ACL (deny-by-default, glob patterns), sandbox enforcer, macOS Keychain integration, SHA-256 audit chain with tamper detection, SecurityGuard middleware, egress firewall, budget enforcer (soft/hard limits), kill switch (Agent/Type/All × Graceful/Emergency), 18 tests
- Plan 726 (P) Artifact Registry: in-memory registry with idempotent upsert, scanner (agents/skills/rules with frontmatter), 4 renderers (Report/VsCode/OpenClaw/API), .github/instructions/ for Rust/Swift/TypeScript, accelerator manifest ADR, convergio-blueprint.yaml format, 9 tests

### Fixed
- plan_reviews table missing spec_file column (broke review registration)
- remember() now exports Markdown backup via with_export_dir()

## [18.2.0] - 26 Marzo 2026

### Added
- TUI Project view tab with project list (T1-01)
- Master plan tree rendering with dependency arrows and expand/collapse (T1-02)
- Plan detail drill-down with hierarchy context bar showing parent + siblings (T1-03)
- Rollup progress bar for master plans with aggregate percentage (T1-04)
- Execution mode badges (SEQ/PAR/MIX/CND) with semantic colors on plan cards (T2-01)
- Delegation status in Mesh view showing peer assignments and progress (T2-02)
- ASCII dependency graph visualization for master plan children (T2-03)
- Project switcher (Ctrl+P) in tab bar with session persistence (T2-04)
- ADR-0115: TUI Project Hierarchy (Plan 719)

## [18.1.0] - 25 Marzo 2026

### Added
- Channel adapters architecture: Slack (Web API + command routing), Email (SMTP relay + subject routing), channel dashboard view with health indicators
- Escalation metric collector for evolution feedback loop
- ADR-0112: Channel Adapters Architecture (Plan 725)
- ADR-0113: Hook Consolidation for Context Window Stability
- ADR-0114: Lean Plan Checkpoints

### Fixed
- Hook consolidation: 13 PreToolUse hooks reduced to 3 via single dispatcher (~77% context event reduction)
- Plan checkpoint v2: lean 4-line format, no sqlite3 direct access, no MEMORY.md mutation
- Route count contract updated for channel API endpoints

## [18.0.0] - 25 Marzo 2026

### Added
- Ecosystem split: convergio-daemon, convergio-app, convergio-web, convergio (meta)
- Resilience framework: circuit breakers, retry, health monitoring, zombie reaper
- Multi-repo orchestration: repositories table, cvg repo CLI
- Cross-project plan dependencies
- Agent IPC optimization (MessagePack)
- Constitution v3.0.0: resilience + swarm articles
- Local LLM watchdog with phone notifications
- Decision audit trail

### Fixed
- 4 VirtualBPM issues (CLI UX, import types, chicken-egg, SQLite locking)

## [17.1.0] — 24 Marzo 2026

### Added
- TUI: Enter drill-down for all views (plan detail, task detail, node info, agent info)
- TUI: Mesh actions — [p]rovision and [h]eartbeat from node popup
- TUI: Agent stop — [s]top from agent popup via IPC unregister
- TUI: Chat delegation via daemon mesh/exec (no API keys, uses logged-in Claude session)
- TUI: Rich popup system with sections, action keys, rounded borders
- TUI: Show all plans toggle ([a] key in kanban)

### Fixed
- TUI: Table column alignment in Pipeline, Mesh, Events, Deliverables views
- TUI: Mesh API parsing for nested {peers:[...]} response format

## [17.0.0] — 24 Marzo 2026

### Added
- Native macOS `CommandCenter` app with onboarding, auth token storage, and SwiftUI routing
- Native Plans, Agents, Mesh, Evolution, Costs, Terminal, and Brain surfaces
- Run cost dashboard backed by `/api/metrics/*` and `/api/runs/*`
- WS-PTY terminal tabs with peer routing, tmux attach/create, and keyboard passthrough
- Brain visualization with realtime `/api/brain` + `/ws/brain` data and Metal-backed rendering
- Native macOS notifications with category preferences and Thor approve/reject actions
- ADR-0110: Plan F Command Center decisions
- TUI: Brain Canvas view with session/agent/task tree visualization
- TUI: Cost/Token Center with model/project/date breakdown
- TUI: Events Stream with action-colored live feed
- TUI: Workspace view with status indicators
- TUI: Deliverables browser with approval status
- TUI: WebSocket client for real-time updates (/ws/brain)
- TUI: --api-url flag for remote daemon connection
- TUI: Maranello ANSI 256-color palette (BG_SURFACE, TEXT_PRIMARY, TEXT_SECONDARY)

### Changed
- Project docs now describe `CommandCenter` alongside the daemon, dashboard, and evolution engine
- Troubleshooting now covers Xcode selection, project generation, PTY session naming, and brain shader behavior
- TUI: Module restructure — views/, widgets/, api/ sub-modules (all under 250 lines)
- TUI: Tab navigation extended to 9 views (keys 1-9)
## [16.0.0] — 23 Marzo 2026

### Added
- Workspace API: /api/workspace/create, /delete, /list, /status, /events, /quality-gate, /release
- Release Agent Rust module — event-driven git export pipeline
- GitConnector trait with GitHub implementation (reqwest)
- Workspace event log (workspace_events table, CRDT-enabled)
- cvg workspace CLI subcommands (create, delete, list, status, events)
- Quality gate module in Rust (replaces pre-merge-gate.sh)
- Wave validation module in Rust (replaces validate-task.sh, validate-wave.sh)
- Non-code deliverables workspace integration
- Hook interception for file ops (workspace-event-hook.sh)

### Changed
- cvg wave create/merge/validate now use workspace layer internally

### Deprecated
- worktree-create.sh — use cvg workspace create-feature
- wave-worktree.sh — use cvg workspace commands
- pr-ops.sh — use cvg workspace release
- pre-merge-gate.sh — use workspace quality gate API
- validate-task.sh, validate-wave.sh — migrated to Rust daemon

## [v15.0.0] - 2026-03-23

### Added
- OpenClaw bridge: Convergio agents accessible via 30+ messaging platforms
- Daemon API: GET /api/openclaw/agents, POST /api/openclaw/invoke
- Skill generator: convergio-openclaw-skills.sh (auto-generates SKILL.md)
- OpenClaw TS plugin: @convergio/openclaw-bridge

## [v14.0.0] - 22 Marzo 2026

### Added
- Plugin activation: `cvg skill enable/disable` manages Claude Code plugins via settings.json
- Domain-aware tool activation: `/solve` Phase 1b detects problem domain, suggests skills
- Domain CLI: `cvg domain list/map` for configurable domain-skill mappings
- CRDT background sync: automatic peer replication on daemon startup (30s default)
- CRDT HTTP endpoints: `/api/crdt/status`, `/api/crdt/peers`, `/api/crdt/force-sync`
- Peer health tracking: unreachable detection after 3 consecutive failures
- MyConvergio `setup.sh`: multi-provider bootstrap with `--rollback` and `--dry-run`
- Skill lint CI: GitHub Action validates skill contributions on PR
- Import auto-defaults: `verify[]` auto-populates `test_criteria` field

### Changed
- mesh-sync.sh scope reduced to git config/scripts only (DB sync via CRDT)
- API task update now writes `notes` and `test_criteria` fields correctly
- `cvg plan validate` uses POST (was GET), syncs wave/plan counters

### Removed
- copilot-sync.sh — replaced by `setup.sh` + `cvg agent sync`

## [v13.0.0] - 22 Marzo 2026

### Added
- Deliverable management: `cvg deliverable create/approve/version` with filesystem output and consent gates
- Project CLI: `cvg project create/list/show` with platform path conventions
- Project audit: `cvg audit --project` CLI + `/api/audit/:project` endpoint with output to project folder
- Agent auto-creation from skill declarations (`requires-agents` field)
- Skill dependency fields: `requires-plugins` and `requires-agents` in skill.yaml
- `cvg skill enable` command for activating skills with dependency resolution
- ADR-0202: Deliverable Management Architecture

### Changed
- Audit output now writes to project-scoped folder instead of global output
- Skill protocol extended with plugin and agent dependency declarations

## [v12.1.1] - 22 Marzo 2026

### Fixed
- `cvg review reset` now accepts optional plan_id (was required, broke planner workflow pre-plan)
- `cvg plan readiness` CLI subcommand added (was API-only, agents couldn't execute workflow step 10)
- `cvg` symlink setup/rollback scripts updated (setup-claude-symlinks.sh, revert-claude-symlinks.sh, convergio-aliases.sh)
- Tests extracted to cli_plan_tests.rs to keep cli_plan.rs under 250 lines

## [v12.1.0] - 22 Marzo 2026

### Added
- cvg plan create/import/start/complete/cancel/approve CLI subcommands
- cvg wave create/merge subcommands
- cvg bus who/send/read/broadcast (IPC commands)
- cvg agent sync/enable/disable/catalog/create/transpile/triage
- cvg metrics summary/collect, cvg alert list, cvg session check
- Plan lifecycle guards (review-create-import-approve-start enforcement)
- Smart import: auto-infer model, validator, effort from task type
- Readiness check endpoint with gates
- Merge-based plan completion metric (waves_merged/waves_total)
- Agent catalog table + multi-provider transpiler
- Agent triage endpoint with keyword scoring
- Mechanical validation gates (credential scan, pattern check, line count, verify commands)
- Thor split: mechanical first, AI judgment at wave level only

### Changed
- Constitution updated to v2.1.0 (6 new principles)
- Rules consolidated from 16 to 12 files (~25% token reduction)
- DRY CLI: all modules use shared crate::cli_http helpers

### Removed
- Bash wrapper (scripts/platform/convergio) replaced by Rust CLI

## [v5.0.0] - 2026-03-22

### Added — Convergio Core Consolidation (Plan #685)
- `cvg` — unified CLI binary (Rust daemon) replacing 400+ sqlite3 calls and bash wrappers
- CLI subcommands: `cvg plan`, `cvg task`, `cvg wave`, `cvg checkpoint`, `cvg lock`, `cvg review`, `cvg agent`, `cvg kb`, `cvg run`, `cvg mesh`, `cvg session`, `cvg skill`, `cvg audit`
- 6 new daemon API endpoints: `POST /api/review`, `POST /api/checkpoint`, `POST /api/kb-write`, `GET /api/path-canonical`, `GET /api/tracking/tokens`, `GET /api/tracking/activity`
- Smart import: daemon infers model, validator, version from spec (no manual flags)
- Tracking API: 4 endpoints for token usage, agent activity, session state, drift detection
- Path canonicalization: case-insensitive file resolution across all CLI and API calls
- `cvg skill lint` and `cvg skill transpile` — replace `skill-lint.sh` + 3 transpiler scripts
- `cvg audit` — replaces `project-audit.sh`
- ADR-0200: Convergio Core Consolidation — single daemon binary as sole state authority

### Changed — BREAKING
- All hooks migrated to `cvg` — zero direct `sqlite3` calls remaining (21 hooks, 7 skills, all rules/docs)
- `plan-db.sh` commands superseded by `cvg plan/task/wave/…` equivalents
- `project-audit.sh` superseded by `cvg audit`
- `skill-lint.sh`, `skill-transpile-*.sh` superseded by `cvg skill lint/transpile`
- Enforcement rules, skill specs, and agent docs updated: all references use `cvg`

### Removed — BREAKING
- Archived: `scripts/archive/mesh-env-setup.sh`, `mesh-normalize-hosts.sh` (zero callers; superseded by daemon)
- Direct sqlite3 in hooks replaced by daemon API calls
- `convergio` bash wrappers for plan/task/wave operations replaced by native Rust CLI

## [v4.0.0] - 2026-03-22

### Added
- CONSTITUTION.md v2.0.0 — 10 articles governing all agents (Constitution, Token Economy, No Professional Advice)
- AgenticManifesto.md — core philosophy (ported from MyConvergio)
- LEGAL_NOTICE.md — comprehensive agentic AI legal notice with attorney-review flags
- SECURITY.md + CONTRIBUTING.md with legal references and CLA
- /solve skill — 10-phase consultant entry point replacing /prompt
- solve_sessions DB migration for decision audit trail
- Persuasion guardrails rule — blocks 10 AI rationalization patterns
- Skill Protocol v1.0 — universal skill format (skill.yaml + SKILL.md)
- 3 transpilers: Claude Code, Copilot CLI, Generic LLM
- skill-lint.sh — CI validation for universal skills
- 7 universal skills: solve, planner, execute, research, check, prepare, release
- AGENTS.md catalog — all agents organized by domain
- acceptance_invariants field in plan spec schema
- 7 rules migrated from MyConvergio (ethical-guidelines, api-development, problem-resolution, agent-discovery, token-budget, lean-coordinator, workflow-enforced)

### Changed
- CLAUDE.md v1.2.0 — added Governance section, Constitution priority
- enforcement.md — /solve as mandatory first step, /prompt deprecated
- MyConvergio restructured as community skills marketplace (Apache 2.0)
- README.md — standalone product documentation

### Removed
- copilot-agents/ directory from MyConvergio (85 files, replaced by transpilers)

## [3.4.0] - 2026-03-21

### Added
- Daemon API: `GET /api/runs` and `GET /api/runs/:id` — execution run history and detail
- Daemon API: `GET /api/metrics` — platform telemetry endpoint (latency, cost, agent count)
- Daemon API: `POST /api/ingest` — document ingestion trigger (PDF/DOCX/XLSX/URL/folder)
- Daemon pause bridge: `POST /api/runs/:id/pause` and `/resume` — suspends execution, preserves state
- CLI thin wrappers: `convergio-run-ops.sh`, `convergio-metrics.sh`, `convergio-ingest.sh` delegate to daemon when available, fall back to sqlite3 with warning when daemon is not running

### Changed
- Bash scripts consolidated into daemon API calls — CLI scripts are now thin wrappers over HTTP
- `convergio-run-ops.sh`: reads `execution_runs` via daemon API; sqlite3 fallback on connection refused

## [3.3.0] - 2026-03-21

### Added
- Document ingestion engine: PDF/DOCX/XLSX/CSV/PPTX/URL/folder → markdown (`scripts/platform/convergio-ingest.sh`)
- `--context` flag on `convergio-run-ops.sh` — attaches ingested documents to execution runs
- Dashboard Approvals view: approve/cancel/pause plans with real-time status (`dashboard/views/approvals.js`)
- `execution_runs` paused status + context columns migration (plan lifecycle tracking)
- Per-run analytics to `convergio-metrics.sh` (duration, cost, agent count per run)

### Changed
- Daemon server files split into 250-line submodules (20+ refactors across mesh/server/ipc/api)
- Evolution engine wiring: convergio-metrics.sh feeds evolution telemetry pipeline
- Dead code removed from autopilot, mesh sync, IPC router

## [3.2.0] - 2026-03-19

### Added
- Real SSE streaming for plan/task progress and WS streaming for terminal/chat (W1)
- CLI TUI live view with tokio event loop for real-time daemon monitoring (W2)
- Menu Bar Mission Control app (SwiftUI + WKWebView) for macOS status bar (W3)
- Chat LLM integration with Claude API and LiteLLM proxy routing (W5)
- Delegation pipeline with real SSH remote spawn for mesh task execution (W6)

### Changed
- Dashboard restructured to 3-zone layout with brain strip, drawers, and evolution panel (W4)
- MyConvergio consolidation: unified settings, preferences, and agent config (W7)
- Replaced simulated SSE/WS endpoints with real streaming implementations (W1)
- Daemon version bumped to 11.6.0

## [3.1.0] - 2026-03-19

### Changed
- Complete dashboard rebuild using MaranelloLuceDesign v4.17.0 Presentation Runtime
- Replaced ~14K LOC vanilla JS with modular ES modules architecture
- All views use Maranello Web Components (mn-chart, mn-data-table, mn-gauge, mn-gantt, mn-modal, mn-tabs)
- Brain neural visualization refactored into 6 modules (max 250 LOC each)
- 4-theme support (Editorial, Nero, Avorio, Colorblind) via mn-theme-rotary
- WCAG 2.2 AA accessibility via mn-a11y FAB
- Mobile responsive with collapsible sidebar

### Removed
- Legacy dashboard JS files (63 files)
- Legacy CSS files (30 files)
- GridStack dependency

## [3.0.0] - 2026-03-18

### Added — Evolution Engine v3
- Telemetry SDK with 7 MetricFamily collectors + SQLite time-series store
- Evaluation engine: 5 domain evaluators (latency, bundle, agent-cost, mesh, workload)
- Hypothesis-driven proposal generator with blast radius classification
- Experiment runner: Shadow/Canary/BlueGreen modes + auto-rollback
- Web research with hypothesis tagging and 7-day cache
- Guardrails: PREnforcer, KillSwitch, RateLimiter, SafetyValidator, AuditTrail
- Cadence: DailyRunner + WeeklyRunner + CadenceScheduler (cron-based)
- Agent profiler + ModelIntelligence + BenchmarkRunner
- MLD CI telemetry feed + NaSra canary adapter
- AutoPilot dashboard: proposals, experiments, agents views
- ROI tracker + scoreboard + NF validation suite (19 tests)
- Architecture docs + ADRs + governance model
- System agents git-tracked in .github/agents/ for cross-machine sync

## [0.1.0] — 2026-03-18

### W1: Scaffold + CI
- Added: repo structure (daemon/, dashboard/, evolution/, scripts/, docs/)
- Added: CI workflow with dashboard, daemon, evolution, constitution checks
- Added: README, CLAUDE.md, LICENSE, ADR-0001

### W2: Dashboard Migration
- Migrated: 494 files from ~/.claude/scripts/dashboard_web/
- Verified: api_server.py, index.html, app.js, all key JS modules

### W3: Daemon + Mesh Merge
- Migrated: 85 .rs files from ~/.claude/rust/claude-core/
- Merged: 15 ConvergioMesh core modules into daemon/src/mesh/
- Resolved: 2 file overlaps (auth.rs, mod.rs)
- Renamed: claude-core → convergio-platform-daemon

### W4: Script Migration
- Moved: 12 mesh scripts → scripts/mesh/
- Moved: 3 platform scripts → scripts/platform/
- Classified: 143 scripts stay in ~/.claude (agent tooling)

### W5: Integration
- Added: DASHBOARD_DB env var for configurable DB path
- Added: start.sh for dashboard and daemon
- Added: .env.example with all config
- Added: migration symlink guide

### W6: Evolution Engine Scaffold
- Added: @convergio/evolution-engine package with TypeScript
- Added: Full type system (Metric, Proposal, Experiment, CapabilityProfile)
- Added: PlatformAdapter interface contract
- Added: 3 adapters (claude, maranello, dashboard)
- Added: Type-shape tests

### W7: Cleanup Strategy
- Added: cleanup-dotclaude.sh (symlink replacement, safe .bak)
- Projected: ~/.claude from 21 GB → ~3.9 GB
- Added: ConvergioMesh deprecation notice
