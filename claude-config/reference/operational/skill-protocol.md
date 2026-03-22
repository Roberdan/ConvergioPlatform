# Skill Protocol v1.0

Universal skill format for AI providers (Claude Code, Copilot CLI, generic LLM).

## Overview

Every skill consists of two files:

| File | Purpose |
|---|---|
| `skill.yaml` | Metadata, configuration, tool requirements |
| `SKILL.md` | Provider-agnostic instruction body |

Transpilers read both files and produce provider-specific output.

## Directory Convention

```
skills/
  solve/
    skill.yaml
    SKILL.md
  planner/
    skill.yaml
    SKILL.md
  check/
    skill.yaml
    SKILL.md
```

## skill.yaml Schema

```yaml
# Skill Protocol v1.0
name: solve                          # unique identifier (snake_case)
version: 1.0.0                       # semver
description: "Problem understanding and triage"
domain: workflow                     # workflow | technical | business | legal | creative
constitution-version: 2.0.0         # minimum Constitution version required
license: MPL-2.0                     # SPDX identifier
copyright: "Roberto D'Angelo 2026"
repo: ConvergioPlatform              # source repository
tools:                               # required tool capabilities
  - Agent
  - Grep
  - Read
  - Write
model: claude-opus-4.6               # recommended model
min-context: 32000                   # minimum context window (tokens)
arguments: optional                  # none | required | optional
triggers:                            # activation patterns
  - "/solve"
  - "help me understand"
provider-formats:                    # transpiler targets
  - claude-code
  - copilot-cli
  - generic-llm
```

### Field Reference

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | yes | Unique identifier, snake_case |
| `version` | semver | yes | Skill version |
| `description` | string | yes | One-line summary |
| `domain` | enum | yes | workflow \| technical \| business \| legal \| creative |
| `constitution-version` | semver | yes | Minimum platform version required |
| `license` | SPDX | yes | e.g. MPL-2.0 |
| `copyright` | string | yes | Author and year |
| `repo` | string | yes | Source repository name |
| `tools` | list | yes | Required tool capabilities |
| `model` | string | yes | Recommended model ID |
| `min-context` | integer | yes | Minimum context window in tokens |
| `arguments` | enum | yes | none \| required \| optional |
| `triggers` | list | yes | Slash command and natural language patterns |
| `provider-formats` | list | yes | Transpiler targets |

## SKILL.md Format

- Provider-agnostic markdown — no provider-specific syntax
- No frontmatter, no `@reference` directives (transpilers add these)
- Uses `{{argument}}` placeholders for dynamic content
- Max 1500 tokens for the skill body
- Required sections:

| Section | Purpose |
|---|---|
| `## Activation` | When and how this skill triggers |
| `## Phases` | Numbered steps the skill executes |
| `## Output` | What the skill produces |
| `## Guardrails` | What the skill must never do |

### Placeholder Syntax

```
{{argument}}          — required argument substitution
{{argument?default}}  — optional argument with default value
{{context}}           — ambient context from caller
```

## Transpiler Contract

Each transpiler reads `skill.yaml` + `SKILL.md` and emits provider-specific output:

| Script | Target | Output Format |
|---|---|---|
| `skill-transpile-claude.sh` | Claude Code | `commands/*.md` with frontmatter |
| `skill-transpile-copilot.sh` | Copilot CLI | `.github/copilot-instructions.md` agent block |
| `skill-transpile-generic.sh` | Any LLM | System prompt string |

Transpilers MUST:
- Inject all `tools` as provider capability declarations
- Substitute `model` into provider model selection
- Expand `triggers` into provider activation syntax
- Validate `constitution-version` against platform version before output

## Versioning Rules

| Change | Version Bump |
|---|---|
| Remove/rename phases, break arguments | major |
| Add phases, new optional fields | minor |
| Fix typos, clarify wording | patch |

`constitution-version` is the minimum platform version required. `skill-lint.sh` enforces this at import time.

## Concrete Example: `check` Skill

### `skills/check/skill.yaml`

```yaml
# Skill Protocol v1.0
name: check
version: 1.0.0
description: "Run lint, type-check, and tests for the current project"
domain: technical
constitution-version: 2.0.0
license: MPL-2.0
copyright: "Roberto D'Angelo 2026"
repo: ConvergioPlatform
tools:
  - Bash
  - Read
model: claude-sonnet-4.6
min-context: 16000
arguments: none
triggers:
  - "/check"
  - "run checks"
provider-formats:
  - claude-code
  - copilot-cli
  - generic-llm
```

### `skills/check/SKILL.md`

```markdown
## Activation
Run when the user issues `/check` or asks to verify project health.

## Phases
1. **Lint** — run linter for the detected language (eslint / ruff / clippy)
2. **Types** — run type checker (tsc --noEmit / mypy / cargo check)
3. **Tests** — run unit tests (vitest / pytest / cargo test)

## Output
- Pass/fail summary for each phase
- Error lines grouped by file
- Exit non-zero if any phase fails

## Guardrails
- NEVER auto-fix lint errors without explicit user approval
- NEVER skip a phase — all three must run
- NEVER suppress errors via inline disable comments
```

## Lint Rules (enforced by `skill-lint.sh`)

- `name` matches directory name
- `version` is valid semver
- `constitution-version` <= platform version
- `SKILL.md` contains all four required sections
- `SKILL.md` body <= 1500 tokens
- No provider-specific syntax in `SKILL.md` (`allowed_tools:`, `@reference`)
- All `tools` entries are known capability identifiers
- `triggers` has at least one slash-command entry (starts with `/`)
