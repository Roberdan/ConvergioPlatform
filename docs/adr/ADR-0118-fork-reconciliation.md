# ADR-0118: Fork Reconciliation — convergio-daemon Standalone vs ConvergioPlatform

**Status**: Accepted
**Date**: 27 Marzo 2026
**Plan**: 734

## Context

Two Rust daemon codebases exist:

| Repo | Location | Files | Last Commit |
|---|---|---|---|
| `convergio-daemon` (standalone) | `~/GitHub/convergio-daemon/` | ~10,591 | 2026-03-25 (`chore: initialize convergio-daemon as standalone repo`) |
| `ConvergioPlatform/daemon/` | `~/GitHub/ConvergioPlatform/daemon/` | ~73,001 | 2026-03-27 (active development) |

The standalone repo was initialised on 2026-03-25 as a local-only experiment (no GitHub remote — `gh repo view Roberdan/convergio-daemon` returns "not found"). ConvergioPlatform contains the kernel, MCP, voice pipeline, workspace, resilience, and all features developed since Plan 700. The standalone repo contains only features up to approximately Wave 2 of an earlier plan.

## Decision

**ConvergioPlatform/daemon/ is the single source of truth.**

The standalone `convergio-daemon` repo will be archived locally (no deletions; kept for reference) and no further development will occur there. All daemon work proceeds exclusively within ConvergioPlatform.

## Rationale

- ConvergioPlatform/daemon/ is 7x larger (73k vs 10k files) and 2 days more recent.
- ConvergioPlatform contains the kernel (ADR-0116), MCP toolchain, Telegram voice pipeline, workspace system, and mesh resilience — none of which exist in the standalone repo.
- A dual-repo strategy would require continuous synchronisation with no benefit: the standalone repo has no active consumers, no GitHub remote, and no CI.
- Keeping one authoritative repo eliminates merge conflicts, simplifies CI, and matches the existing team workflow.

## Consequences

- No action required for ConvergioPlatform — it is already the active repo.
- `~/GitHub/convergio-daemon/` is retained as a read-only local archive; no new commits.
- If a standalone daemon distribution is needed in future, it can be generated via `cargo publish` or a dedicated release workflow from ConvergioPlatform.
- Risk: none — the standalone repo has no GitHub remote, no CI, and no downstream consumers.
