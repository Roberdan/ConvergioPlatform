# ADR-0129: Plan Spec as YAML Before DB Import

**Status**: Accepted
**Date**: 2026-04-01

## Context

The planner generates a plan specification (tasks, waves, dependencies, verification criteria) that must eventually live in the daemon's SQLite database for execution. The question is whether to write directly to the DB during planning or to produce an intermediate YAML artifact first.

Early plans (notably Plan 616) used manual `INSERT INTO tasks` statements, which skipped database triggers, broke internal counters, and bypassed validation. Plan 677 was presented as "ready" but lacked worktree configuration, review linkage, and test criteria — problems that would have been caught by schema validation on a standalone file.

## Decision

The planner MUST produce a `spec.yaml` file as an intermediate artifact. The YAML is reviewed, validated, and approved before any write to the database. The import to DB happens atomically via `cvg plan import`, which enforces invariants.

### Pipeline

```
spec.yaml → schema validation → plan-reviewer agent → fix → cvg plan create → cvg plan import → cvg plan readiness → user approval
```

No data enters the DB until all gates pass.

## Rationale

### 1. Review gate separation

The `plan-reviewer` agent needs a stable, readable artifact to analyze. A YAML file is self-contained and inspectable. Reading plan data back from the DB for review would couple the reviewer to the DB schema and introduce query fragility.

### 2. Multi-layer validation

The YAML passes through independent validation stages:

| Stage | What it checks |
|---|---|
| Schema validation | Every task has `verify[]`, `effort`, `model`, `executor_agent`, `validator_agent`, `output_type` |
| Plan-reviewer agent | Semantic analysis: requirement coverage, dependency soundness, effort realism |
| `cvg plan import` | DB-level: test_criteria presence, effort range, review linkage, worktree creation |
| `cvg plan readiness` | Final check: 0 errors before execution |

Writing directly to the DB would collapse these into a single pass or leave a half-written plan in the DB during validation — dirty state.

### 3. Idempotent iteration

When the reviewer finds problems, the planner edits the YAML and re-validates. This is a simple file edit. If the plan were already in the DB, corrections would require UPDATE statements on N tasks with partial rollback handling — error-prone and non-atomic.

### 4. Atomicity of import

`cvg plan import` writes all tasks in a single transaction with trigger enforcement. Manual inserts (as in Plan 616) skip triggers that maintain wave counters, status constraints, and cross-references. The YAML-first approach ensures the DB is only ever written to through the validated import path.

### 5. Auditability

The YAML file persists as a record of what was planned vs. what was imported. It can be diffed, version-controlled, and referenced in post-mortems. DB state is mutable and lacks this history.

## Consequences

- Every plan requires a file on disk before DB import (minor I/O cost, negligible).
- The `cvg plan import` command is the single entry point to the DB — no manual SQL allowed.
- Plan iteration is fast (edit file, re-validate) without DB cleanup.
- Reviewers operate on a file, not on DB queries, keeping them decoupled from schema changes.

## Incidents that informed this decision

| Incident | What went wrong |
|---|---|
| Plan 616 | Manual `INSERT INTO tasks` skipped triggers, broke counters. Reviews were skipped entirely. |
| Plan 677 | Plan presented as ready but missing worktree, review linkage, and test_criteria. No pre-import validation existed. |
| Plan 10044 | 46 tasks executed and PR created, but plan could not close because Thor was never invoked — downstream consequence of weak gate enforcement. |
