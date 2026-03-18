# Planner Core Workflow

## Activation and Inputs
- Trigger: `/planner` with Opus model.
- Required inputs: user brief, ADRs, CHANGELOG, TROUBLESHOOTING, CI knowledge, prior failures.

## Mandatory sequence
1. Init context and worktree metadata (`planner-init.sh`).
2. Read docs and failed approaches (`plan-db.sh get-failures`).
3. Extract constraints (C-xx) and confirm with user.
4. Clarify technical approach/files/constraints before spec.
5. Generate spec (`spec.yaml` preferred) with explicit `verify` and `consumers` fields.
6. Validate schema before import.
7. Import plan and run intelligence review.
8. Approval gate (explicit yes/proceed).
9. Select parallelization mode.
10. Start plan and execute via `/execute {plan_id}`.

## Gated Plan Creation (NON-NEGOTIABLE)

`plan-db.sh create` and `plan-db.sh import` are **blocked by PreToolUse hook**. The ONLY way to create/import plans is through `planner-create.sh`, which enforces the review exists before allowing creation.

### Mandatory sequence inside planner skill:

```bash
# 1. Launch 1 review agent
Agent(subagent_type="plan-reviewer")  # standard review → /tmp/review-standard.md

# 2. Register review (gate validation)
planner-create.sh register-review standard /tmp/review-standard.md

# 3. Create plan (blocked without review)
planner-create.sh create <project> "<name>" --auto-worktree --human-summary "<summary>"

# 4. Import spec (also blocked without review)
planner-create.sh import <plan_id> spec.yaml

# 5. Reset for next plan
planner-create.sh reset
```

Skipping review = `planner-create.sh` exits with error. No bypass possible.

## Rule highlights
- Never skip F-xx coverage checks.
- Never mark work done without Thor validation.
- Every task must include model, effort, and executor agent.
- No silent exclusions/defer of requirements.
