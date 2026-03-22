# ADR-0201: Plan A — Convergio Core Intelligence

**Status:** Accepted
**Date:** 22 Marzo 2026

## Decision

- CLI POST subcommands via daemon HTTP API (thin client pattern)
- Lifecycle enforcement in daemon guards, not prompt discipline
- Mechanical gates before Thor (save tokens on obvious failures)
- Agent catalog as DB source of truth, files as generated cache
- Rules consolidated from 16 to 12 for ~25% context token reduction

## Consequences

- Daemon rebuild required for new features
- Old bash wrapper removed, cvg is the only CLI entry point
