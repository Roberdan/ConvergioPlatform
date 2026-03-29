# ADR-0124: Fail-Loud Policy

**Status**: Accepted
**Date**: 29 Marzo 2026
**Context**: Plan 756

## Decision

All error handling in the daemon MUST be explicit. Silent error suppression patterns (`.ok()`, `let _ =`, catch-all `_ =>`) are prohibited in production code.

## Context

Audit revealed 446 silent error patterns (239 `.ok()` + 207 `let _ =`) across the daemon. These caused:
- Tasks marked "done" without actual completion (post-mortem evidence)
- Notification delivery failures hidden from callers
- Mesh sync errors silently dropped, causing data inconsistency
- Debug sessions prolonged because errors were invisible

## Rules

1. **DB writes**: propagate with `?` or `map_err` + `tracing::error!`
2. **Network I/O**: `if let Err(e)` + `tracing::warn!` (continue operation)
3. **API responses**: never return `"ok": true` when an operation failed
4. **Legitimate exceptions**: `.parse().ok()` for optional conversions, annotated with `// intentional: <reason>`

## Metrics

| Pattern | Before | After | Reduction |
|---------|--------|-------|-----------|
| `.ok()` | 239 | 21 | -91% |
| `let _ =` | 207 | 12 | -94% |
| Total | 446 | 33 | -93% |

Remaining 33 patterns are legitimate and documented with inline comments.

## Consequences

- Errors are visible immediately, reducing debug time
- Notification `/api/notify` returns per-channel delivery status
- osascript fallback removed — daemon-native notifications only
- Route contract test updated (98 GET, 101 non-GET routes)
