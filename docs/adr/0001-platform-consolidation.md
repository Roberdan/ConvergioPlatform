# ADR 0001: Convergio Platform Consolidation

Status: Accepted | Date: 18 Mar 2026 | Plan: 664

## Context

Platform infrastructure was fragmented across `~/.claude/scripts/dashboard_web/` (115 MB web app), `~/.claude/rust/claude-core/` (Rust daemon), `~/GitHub/ConvergioMesh/` (standalone mesh), and 158 bash scripts. This made `.claude/` grow to 21 GB and mixed config with product code.

## Decision

Create `ConvergioPlatform` as unified repo. Daemon + mesh merge into `daemon/`. Dashboard moves to `dashboard/`. Evolution Engine lives in `evolution/` with standalone core + adapters. `.claude/` returns to config-only.

## Consequences

- Positive: clear separation, independent release cycles, proper CI, reusable evolution core
- Negative: migration effort, path updates needed, temporary symlinks during transition

## Enforcement

- Rule: `.claude/` must stay under 100 MB post-migration
- Check: `du -sh ~/.claude/ | awk '{print $1}'`
