# ADR-0128: Nightly Autonomy Job Architecture

**Status**: Accepted
**Date**: 2026-04-02

## Context

Self-improvement capabilities (goal decomposition, plan generation, agent coordination) need to run on a regular cadence without human intervention. External cron jobs or CI pipelines introduce operational complexity and lose access to daemon-internal state.

## Decision

A nightly autonomy job runs as a scheduled tokio task inside the daemon:
- Fires at configurable time (default: 02:00 local daemon time).
- Invokes the goal decomposer to evaluate pending high-level objectives.
- Applies the risk-based policy engine; actions above threshold queue for approval.
- Actions below threshold execute autonomously and are written to the audit trail.
- Job state (last run, outcomes, errors) persisted in `autonomy_runs` SQLite table.

The job is enabled/disabled via `autonomy.nightly_enabled` in daemon config; disabled by default until the operator explicitly opts in.

## Consequences

**Positive**
- Continuous self-improvement without manual trigger.
- Audit trail provides full visibility into autonomous actions taken overnight.
- Risk gating prevents runaway autonomous changes.

**Negative**
- Operator must review risk thresholds before enabling.
- Long-running goals can cause the nightly job to overlap; mitigated by a job-lock guard.
