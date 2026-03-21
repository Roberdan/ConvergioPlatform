# Changelog

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
