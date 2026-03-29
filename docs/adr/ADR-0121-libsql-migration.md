# ADR-0121: libSQL Migration, Peer Resolver, Evidence Gate Hardening

**Status**: Accepted
**Date**: 29 Marzo 2026
**Plan**: 742 (Plan X v2 — Hardening)

## Context

Plan X v2 addressed three categories of technical debt blocking reliable multi-node execution:

1. **crsqlite instability**: the CRDT extension caused WAL corruption under concurrent writes and required a custom-compiled SQLite fork incompatible with standard rusqlite. Sync failures on M1 Pro were traced to crsqlite version mismatches.
2. **Peer resolution fragility**: delegation scripts used ad-hoc hostname resolution (hardcoded aliases, inconsistent SSH config lookups). Bugs B6 and B9 were both peer-resolution failures.
3. **Evidence gate race conditions**: the kernel verify gate could approve tasks when concurrent writes raced against SHA checks, and zombie reaper shutdown left orphaned lock files.

## Decisions

### 1. Replace crsqlite with timestamp-based sync adapter

Replaced CRDT-based replication with a deterministic timestamp sync model over HTTP. Each row carries `updated_at` (monotonic). Sync resolves conflicts via last-writer-wins with node priority tiebreaker. crsqlite remains as a gated optional dependency but is no longer on the default path.

**Reason**: crsqlite added ~15 MB binary overhead, required manual compilation per architecture, and produced 3 corruption incidents in Plans 729/734. Timestamp sync is simpler, auditable, and sufficient for our 2-3 node topology.

### 2. Centralized peer resolver (3-stage fuzzy match)

Single `resolve_peer()` function used by all delegation and mesh code. Resolution order: (1) exact match in peers.conf, (2) hostname prefix match, (3) Tailscale MagicDNS lookup. All scripts and daemon code route through this resolver.

**Reason**: Bugs B6 and B9 both stemmed from scripts resolving peers differently. A single resolver eliminates the class of bugs.

### 3. Evidence gate hardening

Added mutex around SHA verification to prevent TOCTOU races. Introduced SHA result cache (5-minute TTL) to avoid redundant re-verification. Shutdown sequence now reaps all pending verify locks before exit.

**Reason**: Two tasks in Plan 738 were marked "done" by concurrent verify calls that both read stale state. The mutex + cache eliminates this race.

## Consequences

| Change | Impact |
|---|---|
| Timestamp sync adapter | Breaking: migration required for new columns |
| crsqlite gated | Non-breaking: still loadable via feature flag |
| Peer resolver | All delegation scripts use single resolution path |
| Evidence gate mutex | Slightly higher latency on concurrent verify (~5ms) |
| SHA cache | Reduced I/O on rapid re-verify cycles |

## Migration

Data migration runs automatically on daemon startup (W1, T1-01c). Adds `updated_at`, `sync_node`, `sync_version` columns to plans, tasks, and waves tables. Existing rows backfilled with current timestamp and local node ID.
