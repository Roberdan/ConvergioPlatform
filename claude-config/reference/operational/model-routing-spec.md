<!-- v1.1.0 — Updated for Copilot CLI 1.0.7 + Claude Code 2.1.78 -->

# Model Routing Spec

**Single source of truth for all model IDs, aliases, tiers, and routing decisions.**
CLAUDE.md and README.md reference this file — do not duplicate tables elsewhere.

## Model Registry

| Alias | Full Model ID | Tier | Provider |
|-------|--------------|------|----------|
| `opus` | `claude-opus-4.6` | Premium | Anthropic |
| `opus-1m` | `claude-opus-4.6-1m` | Premium (1M ctx) | Anthropic |
| `sonnet` | `claude-sonnet-4.6` | Standard | Anthropic |
| `haiku` | `claude-haiku-4.5` | Fast/Cheap | Anthropic |
| `codex` | `gpt-5.3-codex` | Standard | OpenAI |
| `codex-mini` | `gpt-5.1-codex-mini` | Fast/Cheap | OpenAI |
| `gpt-5.4` | `gpt-5.4` | Standard (deep reasoning) | OpenAI |
| `gemini-3-pro` | `gemini-3-pro-preview` | Standard (1M ctx) | Google |

> **Note**: In `execute.agent.md` alias mapping, `codex → gpt-5` and `sonnet → claude-sonnet-4.5` are Copilot CLI internal mappings that resolve to the model IDs above at runtime. The canonical IDs in this table are authoritative.

## Phase Routing (CLAUDE.md / workflow)

| Phase | Canonical Model ID | Alias | Agent |
|-------|--------------------|-------|-------|
| Prompt / Requirements | `claude-opus-4.6` | `opus` | `@prompt` |
| Planning | `claude-opus-4.6-1m` | `opus-1m` | `@planner` |
| Plan review (×1) | `claude-sonnet-4.6` | `sonnet` | plan-reviewer |
| Execution / TDD | `gpt-5.3-codex` | `codex` | `@execute` |
| Validation / Thor (wave-only) | `claude-opus-4.6` | `opus` | `@validate` |
| Exploration | `claude-haiku-4.5` | `haiku` | explore |
| Coordinator (default session) | `claude-sonnet-4.6` | `sonnet` | — |

## Task-Type Routing (README.md / ad-hoc)

| Task type | Canonical Model ID | Alias | Rationale |
|-----------|-------------------|-------|-----------|
| Architecture, security review | `claude-opus-4.6` | `opus` | Deep reasoning |
| Standard code generation | `gpt-5.3-codex` | `codex` | Capable, bulk work |
| Deep debugging, design tradeoffs | `gpt-5.4` | `gpt-5.4` | Deep reasoning (OpenAI) |
| Config, mechanical edits | `gpt-5.1-codex-mini` | `codex-mini` | Fast, cheap |
| Documentation | `claude-haiku-4.5` | `haiku` | Fast, trivial |
| Large file / codebase analysis | `claude-opus-4.6-1m` | `opus-1m` | 1M context |
| Large context research | `gemini-3-pro-preview` | `gemini-3-pro` | 1M context, free tier |

## Parallelization & Concurrency

| Scenario | Model |
|----------|-------|
| Standard coordinator (≤3 concurrent) | `claude-sonnet-4.6` |
| Coordinator with max parallel (>3) | `claude-opus-4.6` (required) |
| Task executor (default) | `gpt-5.3-codex` |
| Task executor (cross-cutting/complex) | `claude-opus-4.6` (escalate) |

## Model Selection Decision Tree

```
Need a model?
│
├── Is this a PLANNING step?
│   ├── Full plan decomposition → opus-1m (claude-opus-4.6-1m)
│   └── Single plan review     → sonnet  (claude-sonnet-4.6)
│
├── Is this VALIDATION / THOR?
│   └── Wave-level only        → opus    (claude-opus-4.6)
│
├── Is this CODE EXECUTION / TDD?
│   ├── Standard task          → codex   (gpt-5.3-codex)
│   ├── Cross-cutting / arch   → opus    (claude-opus-4.6)
│   ├── Hard debug / tradeoffs → gpt-5.4
│   └── Mechanical / config    → codex-mini (gpt-5.1-codex-mini)
│
├── Is this EXPLORATION / RESEARCH?
│   ├── Quick lookup           → haiku   (claude-haiku-4.5)
│   ├── Large context analysis → opus-1m (claude-opus-4.6-1m)
│   └── Large context research → gemini-3-pro (gemini-3-pro-preview)
│
└── Default coordinator work   → sonnet  (claude-sonnet-4.6)
```

## Copilot CLI Agent Frontmatter

Canonical model fields used in `copilot-agents/`:

| Agent file | `model:` value |
|------------|----------------|
| `planner.agent.md` | `claude-opus-4.6-1m` |
| `execute.agent.md` | `gpt-5.3-codex` |
| `validate.agent.md` | `claude-opus-4.6` |
| `prompt.agent.md` | `claude-opus-4.6` |
| `code-reviewer.agent.md` | `claude-haiku-4.5` |
| `check.agent.md` | `gpt-5.1-codex-mini` |

## Rules

1. **Aliases are shortcuts** — always resolve to canonical IDs before DB storage or frontmatter.
2. **`sonnet` = `claude-sonnet-4.6`** (not 4.5). Use `claude-sonnet-4.5` only if a specific agent overrides.
3. **Thor is OPUS, always** — no exceptions.
4. **Planner is OPUS-1M** — required for long context plan documents.
5. **Execution default is `codex`** — escalate to `opus` only for cross-cutting/arch tasks.
6. **Haiku for exploration** — never use Haiku for planning, validation, or security.
7. **Never invent model IDs** — only use IDs from the registry above.
8. **`gpt-5.4` for hard debugging** — use when `codex` fails or task requires deep tradeoff reasoning.
9. **`gemini-3-pro` for research only** — NOT for code gen or execution, 1M context advantage.

## CLI Versions & Features (last updated 18 Mar 2026)

| Tool | Version | Key features |
|------|---------|-------------|
| Copilot CLI | 1.0.7 | `/pr`, `/extensions`, GPT-5.4, background multi-turn, tool search |
| Claude Code | 2.1.78 | `/voice`, `/loop`, Opus 4.6 1M, `modelOverrides`, `PostCompact` hook, sparse worktrees |

_Why: Plan 637 — consolidate model routing. Updated v1.1.0 for CLI 1.0.7 + Claude Code 2.1.78._
