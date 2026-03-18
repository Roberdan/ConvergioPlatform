<!-- v3.0.0 -->
# Execution: Testing

## Mock Boundaries (NON-NEGOTIABLE)

ALLOWED: External APIs, Network I/O, File system, Time/Date | FORBIDDEN: Auth functions, DB queries (use test DB), module under test, internal routing. Mock at BOUNDARIES only.

## Integration Tests

New endpoint → real middleware | New API consumer → realistic shape | Interface change → ALL consumers | New CSS vars → in stylesheet. Every plan: >=1 integration test full data path. Assert frontend matches backend. Case-insensitive string enums.

## Fail-Loud (NON-NEGOTIABLE)

`return null` on unexpected empty = BUG. `console.warn` + visible UI. Exception: loading, optional features.

## NON-NEGOTIABLE Test Rules

**Real Data**: Real names/shapes/fields. No `Studio A`/`Test Studio`. Thor 8b + `code-pattern-check.sh`. | **Schema-Migration**: Model change → migration same PR. _PR #235._ | **Signature Change**: Add/remove param → grep ALL callers incl. tests → update. _Plan 100027._ | **Test Domains**: `example.com`/`example.org` (RFC 2606) only. _Plan 100028._ | **Field Addition**: Add field → update ALL test fixtures. _Plans 383+387._

## Test Quality (Thor Gate 8)

Mock depth <=2 | No self-mock | Coverage with assertions | Format matches prod | Consumer tests | Migration exists | Smoke test auth plans | Safe domains

## Lean Coordinator

See `reference/operational/execution-optimization.md`. Recovery: `plan-checkpoint.sh restore` → `plan-db.sh execution-tree`.
