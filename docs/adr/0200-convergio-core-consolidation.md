# ADR-0200: Convergio Core Consolidation — Daemon as Single State Authority

**Status**: Accepted
**Date**: 22 March 2026
**Plan**: #685 — Convergio Core Consolidation (CLI + Daemon Single Source)
**Deciders**: Roberto D'Angelo

---

## Context

ConvergioPlatform had 400+ direct `sqlite3` calls spread across bash hooks, skills, and rules. The `plan-db.sh` family of scripts (21 lib files) duplicated logic already in the Rust daemon. This led to:

- State drift between CLI tools and daemon (race conditions on WAL writes)
- No token tracking: hooks bypassed the daemon, so spend was invisible
- 3 transpiler scripts + `skill-lint.sh` + `project-audit.sh` duplicated Rust capabilities
- Hooks calling `sqlite3` directly instead of the daemon API could corrupt WAL under concurrent access

## Decision

**The Rust daemon binary (`cvg`) is the single source of truth for all plan, task, wave, checkpoint, lock, review, agent, KB, run, mesh, session, skill, and audit operations.**

| Before | After |
|---|---|
| `plan-db.sh list` | `cvg plan list` |
| `plan-db.sh update-task {id} done` | `cvg task update {id} done` |
| `sqlite3 "$DASHBOARD_DB" "SELECT …"` | `cvg plan status {id}` |
| `skill-lint.sh` | `cvg skill lint` |
| `skill-transpile-claude.sh` | `cvg skill transpile claude` |
| `project-audit.sh` | `cvg audit` |

All 21 hooks migrated to daemon API calls. Zero remaining `sqlite3` in hook scripts, skills, or rules.

## Consequences

**Positive:**
- Single WAL writer: no more corruption risk from concurrent sqlite3 processes
- Token tracking: all plan/task lifecycle events flow through daemon, enabling spend attribution
- Path canonicalization: case-insensitive file resolution enforced at API boundary
- 7 skills and all enforcement rules use consistent `cvg` commands
- Smart import: daemon infers model, validator, and version from spec — no manual flags

**Negative (Breaking):**
- All scripts, CI, and documentation that called `plan-db.sh` or direct `sqlite3` must migrate to `cvg`
- Archived: `scripts/archive/mesh-env-setup.sh`, `mesh-normalize-hosts.sh` (zero callers)
- Daemon must be running for write operations; `cvg` fails fast with clear error if unreachable

**Migration:**
- Hooks: `pre-tool-use/` + `post-tool-use/` + lifecycle hooks updated in W4 (21 hooks)
- Skills: 7 universal skills updated in W4 (T4-05)
- Rules + docs: `rules/enforcement.md`, `reference/operational/core-workflow.md`, all agent docs updated in W4 (T4-06)
- Bash fallback for read-only operations (list, status) retained via `cvg` offline mode

## Compliance

- ADR supersedes: none (first CLI unification ADR)
- Monitoring: `cvg tracking tokens` + `cvg tracking activity` for spend attribution
- Rollback: `plan-db.sh` scripts remain in `claude-config/scripts/` for emergency fallback, but are not on `$PATH`

---

*Per Constitution Article V (Quality) and Article IX (Token Economy): a single, validated code path reduces defect surface and agent token waste.*
