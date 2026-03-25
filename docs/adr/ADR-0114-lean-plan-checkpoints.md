# ADR-0114: Lean Plan Checkpoints

## Status: Accepted
## Date: 25 Marzo 2026

## Context
Previous checkpoint implementation injected full task dumps into compacted context, defeating the purpose of compaction. It also used `sqlite3` directly, violating the platform's own hook guards.

## Decision
Plan checkpoint files are max 4 lines: plan name, status, task counts, recovery command. No `sqlite3` direct access, no MEMORY.md mutation. Commit b415b2a.

## Consequences
- Checkpoint files are minimal and safe for context injection after compaction
- All DB access goes through `cvg plan show`, respecting the daemon-first architecture
- MEMORY.md remains stable across sessions (no checkpoint-induced mutations)
- Recovery is a single `cvg checkpoint restore` command
