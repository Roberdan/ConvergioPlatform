# ADR-0126: Conflict-Aware CRDT Sync (HTTP LWW Disabled)

**Status**: Accepted
**Date**: 2026-04-02

## Context

The mesh previously used HTTP Last-Write-Wins (LWW) for state reconciliation. Under concurrent edits from multiple nodes LWW silently discards writes, which caused data loss in plan and task state during high-concurrency scenarios. A deterministic merge strategy was required.

## Decision

Replace HTTP LWW with CRDT vector-clock-based merge:
- Each mutable entity carries a vector clock updated on every write.
- Merge during sync applies CRDT union semantics; concurrent edits are preserved, not dropped.
- HTTP LWW code paths are disabled (`lww_merge` feature flag set to `false`).
- Convergence proofs (deterministic test suites) validate eventual consistency on every CI run.

## Consequences

**Positive**
- No silent data loss under concurrent edits.
- Provable eventual consistency via mesh convergence proof suite.
- Node self-provisioning relies on CRDT to safely bootstrap without a coordinator.

**Negative**
- Vector clocks increase per-entity storage by ~64 bytes.
- Conflicts now surface tombstoned values; callers must handle `Tombstone` variant.
