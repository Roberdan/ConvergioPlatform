# /solve — Problem Understanding Reference

Phases 3–8 of the /solve workflow: triage, clarification, research, F-xx extraction.

## Triage Decision Tree

```
User request → Complexity assessment:
  - Single file change, clear fix        → LIGHT
  - Multiple files, clear scope          → STANDARD
  - Unclear scope / architecture / multi-wave → FULL
```

| Mode | Signals | Output |
|------|---------|--------|
| LIGHT | "fix bug", "rename X", "add field Y", single file mentioned | Direct edit, no plan |
| STANDARD | "add feature", "refactor", "integrate X with Y", 2–5 files | Plan, 1 wave |
| FULL | "redesign", "migrate", "new system", 5+ files, unclear requirements | Plan, multi-wave, Thor |

**Default to STANDARD when in doubt.** Promote to FULL if research reveals hidden scope.

---

## Question Templates by Domain

| Domain | Questions |
|--------|-----------|
| Bug fix | What is the expected vs actual behavior? When did it start? Can you reproduce reliably? Any logs or error messages? |
| New feature | Who is the user? What problem does it solve? What is the acceptance criteria? Any existing similar feature? |
| Refactor | What is the current pain? What should the target look like? What must not break? Is there a deadline? |
| Migration | Source and target? Data volume? Downtime tolerance? Rollback plan? Who owns the rollback? |
| Integration | Which systems? API contract (REST/gRPC/event)? Auth mechanism? Error handling contract? |
| Performance | Current vs target metric? How is it measured? What is the blast radius of changes? |

Ask **at most 3 questions at a time**. Prefer multiple choice over open-ended where possible.
Stop asking when you have enough to write F-xx requirements.

---

## Research Agent Dispatch Patterns

Launch parallel `Explore` agents to gather context before writing requirements.

### Standard dispatch (STANDARD mode)

```
Agent 1 — Codebase: find related files, existing implementations, data models
Agent 2 — Constraints: read CLAUDE.md, rules/, conventions, prior decisions
Agent 3 — Consumers: find all callers/importers of affected symbols
```

### Extended dispatch (FULL mode — add)

```
Agent 4 — Prior art: search for similar patterns already in the codebase
Agent 5 — Risk: identify blast-radius, downstream dependencies, migration needs
```

### Prompt template per agent

```
Explore the codebase. Goal: {specific_question}.
Search paths: {file_globs}.
Return: file paths, relevant excerpts, summary of findings.
Do not implement anything.
```

Collect all findings before writing F-xx. Never dispatch agents after writing requirements.

---

## F-xx Extraction Format

```yaml
requirements:
  - id: F-01
    text: "exact user words + inferred wiring"
    wave: W1
    acceptance: "machine-verifiable criterion"
    priority: must|should|nice-to-have

  - id: F-02
    text: "..."
    wave: W1
    acceptance: "..."
    priority: must
```

Rules:
- `text`: quote the user where possible; add wiring inference in brackets
- `acceptance`: must be verifiable by a command or assertion (curl, test, grep, wc)
- `wave`: assign based on dependency order, not just time
- `priority`: `must` = blocks release, `should` = in scope but flexible, `nice-to-have` = drop if time-boxed

---

## Acceptance Invariant Examples

```yaml
acceptance_invariants:
  - description: "API returns 200 for valid input"
    verify: "curl -s localhost:8420/api/endpoint | jq .status"
    defined_by: collaborative

  - description: "No TypeScript errors after change"
    verify: "cd evolution && npx tsc --noEmit"
    defined_by: technical

  - description: "File count under 250 lines"
    verify: "wc -l {file} | awk '{if($1>250) exit 1}'"
    defined_by: convention

  - description: "All existing tests pass"
    verify: "cd daemon && cargo test 2>&1 | tail -1"
    defined_by: regression
```

`defined_by` values: `collaborative` (from user), `technical` (from research), `convention` (from rules).

---

## Reformulation Protocol

**When to reformulate** (requires explicit user confirmation before proceeding):

| Signal | Example |
|--------|---------|
| Stated problem is a symptom | "fix login bug" → auth token expiry is the root cause |
| Scope larger than expected | "add field" → requires DB migration + API change + frontend update |
| Existing solution found | "build X" → X already exists at `path/to/x.ts`, needs extension not rebuild |
| Conflicting requirements | F-01 and F-03 cannot both be true simultaneously |

**How to reformulate:**

1. Present research findings concisely (bullet list, no prose)
2. State what you now believe the real problem is
3. Propose a new problem statement
4. Ask: "Should I proceed with this reframing, or keep the original scope?"
5. Wait for explicit confirmation before writing F-xx or creating a plan

**Never silently reframe.** A reformulation that skips confirmation wastes a full wave.

---

## Phase Summary (3–8)

| Phase | Action | Output |
|-------|--------|--------|
| 3 | Triage | LIGHT / STANDARD / FULL |
| 4 | Clarify | 3 questions max, answers recorded |
| 5 | Research | Parallel Explore agents dispatched |
| 6 | F-xx | Requirements list with acceptance criteria |
| 7 | Reformulate? | If needed: present findings, confirm |
| 8 | Hand off | /planner (STANDARD/FULL) or direct edit (LIGHT) |
