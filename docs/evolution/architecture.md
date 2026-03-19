# Evolution Engine v3 — Architecture

## Overview

The Convergio Evolution Engine v3 is a self-optimisation platform that continuously
improves the Convergio mesh by collecting telemetry, evaluating system health, generating
hypothesis-driven proposals, running controlled experiments, and applying validated changes
through guardrail-protected pathways. All production changes are gated behind PR-based review
and experiment validation — the engine never writes directly to production state.

The engine operates on two cadences: a **daily scan** (06:00 UTC weekdays) that collects
metrics and evaluates anomalies, and a **weekly deep evaluation** (Sunday 02:00 UTC) that
aggregates trends and generates strategic proposals. Proposals progress through a lifecycle
from Draft → Shadow → Canary → Approved → Applied, with blast-radius-based approval gates
enforced by the Guardrails layer.

Each component is adapter-agnostic: platform-specific I/O (metrics, PR submission, canary
traffic) is encapsulated in `PlatformAdapter` implementations. The engine core deals only
with typed interfaces, making it testable in isolation from any live infrastructure.

## data flow

```
Collectors          Telemetry Store       Evaluators
[Agent]    ──┐      ┌─────────────┐      ┌──────────────────┐
[Runtime]  ──┤ ───► │  SQLite     │ ───► │ LatencyEvaluator │ ──┐
[Workload] ──┤      │  telemetry_ │      │ BundleEvaluator  │   │
[Mesh]     ──┤      │  metrics    │      │ AgentCostEval.   │   │
[Database] ──┤      │  (5-min     │      │ MeshEvaluator    │   │
[Build]    ──┤      │   rollups)  │      │ WorkloadEval.    │   │
[Bundle]   ──┘      └─────────────┘      └──────────────────┘   │
                                                                  │
◄─────────────────────────────────────────────────────────────── ┘
│  EvaluationResult { anomalies[], opportunities[] }
│
▼
ProposalGenerator                    Guardrails
┌──────────────────────┐            ┌───────────────────┐
│ HypothesisStore      │            │ PREnforcer        │
│ ScoringModel         │ ─────────► │ KillSwitch        │
│ ProposalGenerator    │  proposals │ RateLimiter       │
└──────────────────────┘            │ SafetyValidator   │
                                    │ AuditTrail        │
                                    └─────────┬─────────┘
                                              │ approved
                                              ▼
                                    ExperimentRunner
                                    ┌───────────────────┐
                                    │ Shadow mode       │
                                    │ Canary mode       │ ──► auto-rollback
                                    │ BlueGreen mode    │
                                    └───────────────────┘
                                              │ result
                                              ▼
                                    PlatformAdapter.applyProposal()
                                    (PR creation / mesh config patch)
```

## Component Table

| Component | Path | Responsibility |
|---|---|---|
| EvolutionEngine | `evolution/core/engine.ts` | Top-level orchestrator; collect → evaluate → propose → experiment cycle |
| MetricStore | `evolution/telemetry/store.ts` | SQLite persistence for raw metrics; 5-min rollup aggregation |
| CollectorRegistry | `evolution/telemetry/collectors.ts` | Registry of `MetricCollector` instances; fan-out collection |
| AgentMetricCollector | `evolution/telemetry/collectors/agent-collector.ts` | Collects token spend, session count, cost.usd from dashboard.db |
| BaseEvaluator | `evolution/core/evaluators/base-evaluator.ts` | Abstract evaluator; filters metrics by family, calls `analyze()` |
| LatencyEvaluator | `evolution/core/evaluators/domains/latency-evaluator.ts` | HTTP p95 latency anomaly detection + caching opportunities |
| BundleEvaluator | `evolution/core/evaluators/domains/bundle-evaluator.ts` | JS bundle size regression detection |
| AgentCostEvaluator | `evolution/core/evaluators/domains/agent-cost-evaluator.ts` | LLM token cost trend analysis |
| MeshEvaluator | `evolution/core/evaluators/domains/mesh-evaluator.ts` | Mesh peer health, sync lag, connectivity anomalies |
| WorkloadEvaluator | `evolution/core/evaluators/domains/workload-evaluator.ts` | CPU/memory saturation, queue depth anomalies |
| ProposalGenerator | `evolution/core/analysis/proposal-generator.ts` | Converts opportunities into scored, blast-radius-classified proposals |
| HypothesisStore | `evolution/core/analysis/hypothesis-store.ts` | Persists research hypotheses; links proposals to outcomes |
| OutcomeTracker | `evolution/core/evaluators/outcome-tracker.ts` | Records experiment outcomes; feeds hypothesis validation |
| KillSwitch | `evolution/core/guardrails/kill-switch.ts` | Emergency halt; isEnabled() gates all engine activity |
| RateLimiter | `evolution/core/guardrails/rate-limiter.ts` | Enforces proposals/day, proposals/week, token budget |
| DailyRunner | `evolution/core/cadence/daily-runner.ts` | Cron-triggered daily scan; reads budget before executing |
| WeeklyRunner | `evolution/core/cadence/weekly-runner.ts` | Cron-triggered deep evaluation; produces strategic proposals |
| CadenceScheduler | `evolution/core/cadence/cadence-scheduler.ts` | node-cron wrapper; manages DailyRunner + WeeklyRunner schedules |
| ClaudeConfigAdapter | `evolution/adapters/claude-adapter.ts` | Metrics from ~/.claude/data; PR creation via GitHub CLI |
| MLDAdapter | `evolution/adapters/mld-adapter.ts` | MaranelloLuceDesign CI telemetry (build times, bundle sizes) |
| DashboardAdapter | `evolution/adapters/dashboard-adapter.ts` | Reads/writes JSON data files for dashboard_web |
| RoiTracker | `evolution/reporting/roi-tracker.ts` | Weekly ROI summary: experiments, savings, delta scores |
| Scoreboard | `evolution/reporting/scoreboard.ts` | Top-N proposals ranked by delta score |

## integration points

| System | Interface | Data Direction |
|---|---|---|
| **ConvergioMesh** | `mesh-peers` SQLite table, `mesh-sync-all.sh` | Reads peer health metrics; writes config proposals via PR |
| **dashboard_web** | JSON data files in `data/`, evolution widgets in `dashboard_web/evolution/` | Engine writes snapshot JSON; widgets read and render |
| **GitHub** | `gh pr create` via PlatformAdapter | Proposals with blast radius ≥ SingleRepo require PR approval |
| **NaSra** | `NaSraCanaryAdapter` | Canary traffic routing for web experiments |
| **MLD CI** | `MLDAdapter` telemetry collector | Build duration, bundle size metrics from CI artifacts |

## Deployment Topology

```
M1 (primary)                         M5 (peer)
┌────────────────────────────┐        ┌─────────────────────────┐
│ EvolutionEngine (active)   │        │ MetricStore (replica)   │
│ CadenceScheduler (running) │        │ MeshEvaluator (passive) │
│ DailyRunner @ 06:00 UTC    │ ──────► │ mesh-sync-all.sh target │
│ WeeklyRunner @ 02:00 Sun   │  sync  │                         │
│ SQLite: telemetry.db       │        │ SQLite: telemetry.db    │
│ SQLite: evolution.db       │        └─────────────────────────┘
└────────────────────────────┘
         │
         │ writes
         ▼
   dashboard_web/data/
   (evolution-snapshot.json,
    roi-summary.json)
```

**Sync mechanism**: `mesh-sync-all.sh` replicates telemetry and config files to M5.
The engine runs exclusively on M1; M5 provides read-only metric redundancy and
serves as a canary target for mesh-topology experiments.
