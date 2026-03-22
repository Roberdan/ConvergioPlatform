<!-- v2.0.0 — Merged: docs-troubleshooting + problem-resolution -->
# Documentation & Troubleshooting

## Code Docs

JSDoc/docstrings public APIs, WHY not WHAT | Module: README.md/package | API: OpenAPI | ADRs: `/docs/adr/`, numbered

## Per-Wave ADR (NON-NEGOTIABLE)

Every wave → ADR. Thor Gate 9. CHANGELOG: `## [vX.Y.Z] - date` → `### Added|Changed|Fixed`, 1-line entries.

## TROUBLESHOOTING.md (NON-NEGOTIABLE)

Every repo root. Update every plan. Format: `## Problem:` → `**Symptom/Cause/Fix**:`.

## Problem Resolution Protocol (NON-NEGOTIABLE)

BEFORE fixing errors, agents MUST follow this search order:

| Step | Source | Command/Action | Skip if |
|---|---|---|---|
| 1 | Repo `TROUBLESHOOTING.md` | `Read TROUBLESHOOTING.md` (root) | File doesn't exist |
| 2 | Repo ADRs | `Glob("docs/adr/*.md")` + `Grep(pattern="keyword", path="docs/adr/")` | No `/docs/adr/` dir |
| 3 | Global KB | `plan-db.sh kb-search "error keywords" --limit 5` | Empty results |
| 4 | Global troubleshooting | `Read claude-config/` (if relevant docs exist) | Dir doesn't exist |
| 5 | Web/Explore | `WebSearch` or `Task(subagent_type="Explore")` | Steps 1-4 resolved |

Rules: **NEVER attempt a fix without completing steps 1-2 first** | Cite source when applying fix | Update TROUBLESHOOTING.md after new resolution | KB write: `plan-db.sh kb-write troubleshooting "title" "solution" --tags '["error-type"]'`

## Problem Resolution Anti-Patterns

| WRONG | RIGHT |
|---|---|
| Immediately try Stack Overflow fix | Check repo TROUBLESHOOTING.md first |
| Guess based on error message | Search ADRs for prior decisions on this area |
| Retry same approach 3 times | Check `plan-db.sh get-failures $PROJECT_ID` for prior failures |
| Fix without documenting | Add to TROUBLESHOOTING.md after resolution |
