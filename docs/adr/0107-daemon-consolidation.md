# ADR 0107 — Daemon Consolidation: Bash → Daemon API

**Date:** 2026-03-21
**Status:** Accepted

## Context

CLI scripts (`convergio-run-ops.sh`, `convergio-metrics.sh`, `convergio-ingest.sh`) queried SQLite directly via `sqlite3`, bypassing the daemon and its concurrency guarantees (WAL, CRDT). This caused read conflicts when the daemon was writing, and duplicated query logic across Bash and Rust.

## Decision

Move all data access behind daemon HTTP endpoints (`/api/runs`, `/api/metrics`, `/api/ingest`, `/api/runs/:id/pause`). CLI scripts become thin wrappers: they POST/GET to the daemon when it is reachable on `:8420`, and fall back to read-only `sqlite3` with a stderr warning when the daemon is not running.

## Consequences

- Single source of truth for all data mutations — no more direct SQLite writes from Bash.
- Fallback mode preserves observability (read-only) without requiring the daemon for basic queries.
- New API surface (`runs`, `metrics`, `ingest`, `pause`) must be kept in sync with the daemon route registry.
- Scripts that previously worked without a running daemon now emit a warning; operators must start the daemon for write operations.
