# ADR 0299 — Resilience Framework

**Date**: 25 Marzo 2026
**Status**: Accepted
**Plan**: 724 / Wave W2

## Context

Plan 724 introduced a resilience mandate (CONSTITUTION.md Article XI) requiring every daemon
component to self-recover from failures. Prior to W2, the daemon had no circuit breakers, no
retry logic, and no zombie-reaper — a single mesh node failure could cascade into DB locks and
orphaned worktrees.

Inspired by HPC distributed systems (fault tolerance as first-class property), the framework
provides: circuit breakers, retry with exponential backoff, health registry, zombie reaper,
and checkpoint/restart.

## Decision

Implement `daemon/src/resilience/` with five modules:

| Module | Responsibility |
|---|---|
| `circuit_breaker.rs` | Closed/Open/HalfOpen state machine for external boundaries |
| `retry.rs` | `retry_with_backoff<F,T,E>()` — jittered exponential backoff |
| `health.rs` | `HealthCheck` trait, `HealthRegistry`, `/api/health/deep` endpoint |
| `reaper.rs` | Stale worktree, merged branch, and lock-file cleanup |
| `checkpoint.rs` | Plan-state snapshots; safe restart after crash |

CLI: `cvg reap [--dry-run]` | `cvg checkpoint save/restore <plan_id>`

## Parisi Model (Swarm Intelligence — Article XII)

The resilience topology is inspired by Giorgio Parisi (Nobel Physics 2021): emergent order from
local interactions. Each daemon component enforces health independently; the mesh reorganizes
without a coordinator. No single component failure halts the swarm.

| Pattern | Daemon Implementation |
|---|---|
| Local rules | Each module handles its own failure without global lock |
| Emergent recovery | Reaper + circuit breakers restore topology automatically |
| Observer safety | Health checks are read-only and do not mutate execution state |
| No SPOF | Any node failure triggers swarm reorganization via mesh coordinator |

## Circuit Breaker Configuration

| Parameter | Default | Meaning |
|---|---|---|
| `failure_threshold` | 3 | Consecutive failures to open |
| `reset_timeout` | 30 s | Time in Open before HalfOpen probe |

Applied at: HTTP/SSH mesh calls, external API calls, filesystem writes.

## Retry Configuration

| Parameter | Default | Meaning |
|---|---|---|
| `max_retries` | 3 | Max attempts |
| `initial_delay` | 100 ms | First backoff |
| `max_delay` | 2 000 ms | Cap |
| `backoff_factor` | 2.0 | Multiplier per retry |
| Jitter | ±20 % | Prevents thundering herd |

Applied at: SQLite busy errors (replaces raw `SQLITE_BUSY` propagation).

## Health Monitoring

`GET /api/health/deep` returns `ComponentHealth` for:
- `database` — connection + WAL status + busy check
- `filesystem` — data dir writable + disk space
- `ipc_engine` — initialized or not
- `swarm` — peer count + last heartbeat age

Liveness (`/api/health`) remains lightweight; deep health is for watchdog and CI.

## Zombie Reaper

Auto-runs every 30 min via `tokio::spawn`. Removes:
- Stale worktrees (no matching wave/plan, or plan done > 24 h ago)
- Merged branches (`git branch --merged main`)
- Lock files in `/tmp` idle > 1 h

Manual: `cvg reap --dry-run` previews; `cvg reap` executes.

## Checkpoint/Restart

`checkpoint` table (`plan_id`, `wave_id`, `state JSON`, `created_at`).
Save: `cvg checkpoint save <plan_id>` → POST `/api/plan-db/checkpoint/save`
Restore: `cvg checkpoint restore <plan_id>` → GET `/api/plan-db/checkpoint/restore`
Used by coordinator after compaction or crash.

## Consequences

**Positive**: cascading failures eliminated; orphan cleanup automated; CI deep health gate
available; restart after crash is safe.

**Negative**: retry adds latency on transient failures (~200 ms average). Acceptable given
alternative is propagated error.

**Constraints**: circuit breaker state is in-process (resets on daemon restart). Persistent
state across restarts is out of scope for this ADR.

## Files

`daemon/src/resilience/mod.rs` | `circuit_breaker.rs` | `retry.rs` | `health.rs`
`reaper.rs` | `checkpoint.rs` | `notify.rs` | `watchdog.rs`
