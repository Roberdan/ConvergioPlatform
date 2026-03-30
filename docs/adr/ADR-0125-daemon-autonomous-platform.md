# ADR-0125: Daemon as Autonomous Platform

**Status**: Accepted
**Date**: 2026-04-02

## Context

ConvergioPlatform evolved from a static orchestration tool into a self-improving system requiring autonomous decision-making, scheduled background work, and goal decomposition. Running these capabilities inside the existing daemon process avoids a separate scheduler binary and keeps the operational model simple (one process to monitor and restart).

## Decision

The Rust daemon hosts the autonomous platform capabilities:
- A built-in cron scheduler executes the nightly autonomy job without external tooling.
- A goal decomposer converts high-level objectives into concrete tasks stored in the plan DB.
- A risk-based policy engine gates autonomous actions on configurable risk-score thresholds.
- An approval UX flow surfaces high-risk decisions to a human operator via IPC before execution.

## Consequences

**Positive**
- Single deployment artifact; no Python or Node sidecar required.
- Autonomous actions share the daemon's SQLite WAL transaction guarantees.
- IPC audit trail captures every autonomous decision with agent identity and timestamp.

**Negative**
- Autonomous job failures can affect daemon stability; mitigated by tokio task isolation and rollback snapshots.
- Risk thresholds are config-file values; wrong tuning may over- or under-block.
