# Changelog

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
