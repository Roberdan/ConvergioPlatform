# Spec YAML Schema

Reference for plan spec YAML files imported via `cvg plan import <plan_id> spec.yaml`.

## Quick Start

```bash
cvg plan template          # print full example with all fields
cvg plan import 42 spec.yaml  # import into plan 42
```

## Top-Level Structure

```yaml
waves:
  - id: "W1"
    name: "Wave Name"
    tasks:
      - id: "T1-01"
        title: "Task description"
```

## Wave Fields

| Field | Type | Required | Default | Notes |
|-------|------|----------|---------|-------|
| `id` | string | yes | — | Unique wave identifier (e.g. "W1") |
| `name` | string | yes | — | Wave name. Alias: `title` |
| `depends_on` | string | no | null | Wave dependency (e.g. "W1") |
| `estimated_hours` | integer | no | 8 | Estimated hours for the wave |
| `tasks` | array | no | [] | List of TaskSpec objects |

## Task Fields

| Field | Type | Required | Default | Notes |
|-------|------|----------|---------|-------|
| `id` | string | yes | — | Unique task ID (e.g. "T1-01") |
| `title` | string | yes | — | Task title. Aliases: `do`, `summary` |
| `type` | string | no | "feature" | feature, bugfix, test, planning, analysis, review, docs |
| `priority` | string | no | "P1" | P0, P1, P2 |
| `description` | string | no | null | Detailed description |
| `model` | string | no | auto | Inferred from type (see below) |
| `assignee` | string | no | null | copilot, claude, or null |
| `output_type` | string | no | "pr" | pr, document, analysis, design, legal_opinion |
| `validator_agent` | string | no | auto | Inferred from output_type (see below) |
| `effort_level` | integer | no | auto | 1-3, inferred from file count |
| `files` | array | no | [] | Files modified by this task |
| `verify` | array | no | auto | Verify commands (auto from files) |
| `test_criteria` | string/object | no | auto | Test criteria (auto from verify) |

## Auto-Inference Rules

### Model (from `type`)

| Task Type | Model |
|-----------|-------|
| test, planning, analysis, review | claude-opus-4.6 |
| all others | gpt-5.3-codex |

### Validator (from `output_type`)

| Output Type | Validator |
|-------------|-----------|
| pr, review | thor |
| document, presentation | doc-validator |
| analysis, plan | plan-reviewer |
| design | design-validator |
| legal_opinion | compliance-validator |

### Effort Level (from file count)

| File Count | Effort | Floor |
|------------|--------|-------|
| 0 | 2 | planning/analysis: 2 |
| 1 | 1 | planning/analysis: 2 |
| 2-4 | 2 | — |
| 5+ | 3 | — |

### Verify Commands

If `files` is non-empty and `verify` is empty, auto-generates `test -f <file>` per file.

### Test Criteria

If `test_criteria` is absent and `verify` has entries, generates `cmd1 AND cmd2 AND ...`.

## Input Formats

The import endpoint accepts three formats:

1. **YAML string** (recommended): `{"plan_id": N, "spec": "<yaml>"}`
2. **JSON waves array**: `{"plan_id": N, "waves": [...]}`
3. **JSON spec object**: `{"plan_id": N, "spec": {"waves": [...]}}`

## Minimal Example

```yaml
waves:
  - id: "W1"
    name: "Core"
    tasks:
      - id: "T1-01"
        title: "Build the thing"
        files:
          - src/thing.rs
```

Everything else is inferred: type=feature, priority=P1, model=gpt-5.3-codex, validator=thor, effort=1, verify=`test -f src/thing.rs`.
