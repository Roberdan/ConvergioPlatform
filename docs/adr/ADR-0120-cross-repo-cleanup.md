# ADR-0120: Cross-Repo Cleanup — Duplicate Archives and Plan R Cancellation

**Status**: Accepted
**Date**: 27 Marzo 2026
**Plan**: 737 (Wave 1, Task T1-01)

## Context

Audit of active repositories and plans identified four sources of duplication and technical debt:

1. `Roberdan/convergio-daemon` (GitHub) — duplicate of `ConvergioPlatform/daemon/`. ADR-0118 established `ConvergioPlatform/daemon/` as the single source of truth, but the standalone GitHub repo remained unarchived.
2. `Roberdan/convergio-app` (GitHub) — SwiftUI CommandCenter app, deprecated per its own README. `convergio-web` (Tauri + Next.js) supersedes it per ADR-0117.
3. Plan R (Plan 731) — tracked development of a SwiftUI CommandCenter. `convergio-web` renders this plan obsolete.
4. `ConvergioPlatform/gui/` and `ConvergioPlatform/dashboard/` — legacy directories to be removed in T2-01.

## Decisions

### 1. Archive `Roberdan/convergio-daemon`

The standalone GitHub repo is now archived (read-only). No deletions; retained for historical reference.
**Reason**: `ConvergioPlatform/daemon/` is 7x larger and contains all active features (kernel, MCP, voice pipeline, workspace, resilience). See ADR-0118 for full fork-reconciliation rationale.

### 2. Archive `Roberdan/convergio-app`

The SwiftUI CommandCenter GitHub repo is now archived (read-only).
**Reason**: `convergio-web` (Tauri + Next.js, per ADR-0117) replaces it. Native macOS app strategy targets WWDC June 2026 with a full native rewrite (Plan 711 decision).

### 3. Cancel Plan R (Plan 731)

Plan 731 (convergio-app / CommandCenter development) cancelled with reason: "Replaced by convergio-web (Tauri + Next.js). SwiftUI CommandCenter archived."
All 6 pending tasks cancelled automatically.

### 4. Scope: Remaining Cleanup (T2-01)

The following will be addressed in Task T2-01 of Plan 737:
- Remove `ConvergioPlatform/gui/` directory (superseded by `convergio-web/`)
- Remove `ConvergioPlatform/dashboard/` directory (superseded by `convergio-web/`)
- Rename `claude_core` → `convergio_core` library

## Consequences

| Item | State |
|---|---|
| `Roberdan/convergio-daemon` | Archived on GitHub |
| `Roberdan/convergio-app` | Archived on GitHub |
| Plan 731 | Cancelled (6 tasks) |
| `ConvergioPlatform/daemon/` | Active SoT (unchanged) |
| `convergio-web/` | Active replacement for app + dashboard |
| `ConvergioPlatform/gui/` | Pending removal (T2-01) |
| `ConvergioPlatform/dashboard/` | Pending removal (T2-01) |
| `claude_core` lib | Pending rename to `convergio_core` (T2-01) |

## Naming Decisions (Preserved)

### npm Scope

- **Official scope**: `@convergio` — all packages published under this scope.
- `@maranello` references in external repos (e.g., convergio-design, convergio-community) should be updated to `@convergio` **when those files are next touched**. No bulk rename required now.
- **Plan 733 (Monorepo Split)**: spec currently references `@maranello` packages — must be updated to `@convergio` before execution.

### CSS Variables

- **Prefix**: `--mn-*` retained as-is (Maranello brand identity).
- Renaming to `--cv-*` or similar is not planned: cost/benefit unfavorable (hundreds of references, zero functional gain, high breakage risk).
- This decision is final for Plan 737 scope.

### Summary Table

| Artifact | Decision | Rationale |
|---|---|---|
| npm scope | `@convergio` (official) | Brand alignment |
| CSS variables | `--mn-*` (kept) | Too many references, brand identity |
| `@maranello` in other repos | Update when touched | Incremental, low risk |
| Plan 733 spec | Must update to `@convergio` | Pre-execution requirement |
