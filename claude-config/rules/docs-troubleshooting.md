# Documentation & Troubleshooting

## Code Docs

JSDoc/docstrings public APIs, WHY not WHAT | Module README | OpenAPI for APIs | ADRs: `/docs/adr/`, numbered

## Per-Wave ADR (NON-NEGOTIABLE)

Every wave → ADR. Thor Gate 9. CHANGELOG: `## [vX.Y.Z] - date` → `### Added|Changed|Fixed`, 1-line entries.

## TROUBLESHOOTING.md (NON-NEGOTIABLE)

Every repo root. Update every plan. Format: `## Problem:` → `**Symptom/Cause/Fix**:`.

## Problem Resolution (NON-NEGOTIABLE)

Search order — NEVER fix without completing steps 1-2:

| # | Source | Action |
|---|---|---|
| 1 | Repo TROUBLESHOOTING.md | `Read TROUBLESHOOTING.md` |
| 2 | Repo ADRs | `Grep(pattern, path="docs/adr/")` |
| 3 | Global KB | `cvg plan --help` |
| 4 | Global docs | `Read claude-config/` |
| 5 | Web/Explore | `WebSearch` or Explore agent |

Cite source when applying fix. Update TROUBLESHOOTING.md after new resolution.
