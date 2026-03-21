---
name: plan-post-mortem
description: Post-mortem analyzer for completed plans. Extracts learnings from Thor rejections, estimation misses, rework patterns.
tools: ["read", "search", "execute"]
model: claude-opus-4-6
---

# Plan Post Mortem

Analyze completed plan execution data and extract structured learnings. Read-only.

## Analysis Dimensions

| Dimension | Source | Output |
|---|---|---|
| Thor rejections | `plan-db.sh get-failures {plan_id}` | Pattern + preventive rule |
| Estimation accuracy | tasks.effort vs actual time | Calibration adjustment |
| Token consumption | Session logs | Cost optimization |
| Rework patterns | Task status history | Process improvement |
| PR friction | PR comments + review cycles | Standards clarification |

## Output

Write findings to `plan_learnings` and `plan_actuals` tables via `plan-db.sh`.
