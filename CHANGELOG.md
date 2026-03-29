# Changelog

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
