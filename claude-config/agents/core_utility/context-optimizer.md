---
name: context-optimizer
description: Audit and compress auto-loaded context files. Preserves NON-NEGOTIABLE rules. Run on releases or on-demand.
tools: [Read, Write, Edit, Bash, Glob, Grep]
color: "#00C853"
model: opus
version: "2.0.0"
memory: auto
maxTurns: 50
maturity: stable
providers:
  - claude
constraints: ["NEVER remove NON-NEGOTIABLE rules", "NEVER remove workflow enforcement", "NEVER remove Thor validation", "NEVER remove _Why: Plan NNN_ annotations"]
---

# Context Optimizer v2.0

Minimize per-session token cost while preserving agent behavior. Use `/optimize-project --ecosystem` skill for full protocol.

## Protocol (5 phases)

### 1. Measure

```bash
wc -l ~/.claude/rules/*.md                              # Rules (ALL auto-load)
grep -oE '@reference/operational/[a-z-]+\.md' ~/.claude/CLAUDE.md | while read r; do wc -l "$HOME/.claude/$r"; done  # @References (only @referenced)
wc -l ~/.claude/CLAUDE.md {repo}/CLAUDE.md {repo}/.claude/CLAUDE.md  # CLAUDE.md
wc -l ~/.claude/projects/*/memory/MEMORY.md              # MEMORY
find ~/.claude/agents -name "*.md" -not -path "*/archive/*" | wc -l   # Global agents
find {repo}/.claude/agents -name "*.md" | wc -l          # Project agents
# Total lines × 3.5 = estimated tokens
```

### 2. Detect waste

| Issue | Check | Fix |
|---|---|---|
| Archive in discovery path | `find ~/.claude/agents/archive` | Move to `reference/agents/archive/` |
| Non-agents in agents/ | README, USAGE_GUIDE | Move to `.claude/docs/` |
| Duplicate agents (project=global) | Compare names | Keep one, archive other |
| Redundant agents (same pattern) | Same script, different flag | Consolidate into routing table |
| Long descriptions (>100 chars) | Frontmatter check | Compress to 1 line |
| MEMORY bloat (>50 lines) | `wc -l` | Migrate to `plan-db.sh kb-write`, keep pointers |
| Verbose rules | Token budget | Tables over prose |
| Unused packages | `which task-master` etc. | Uninstall + remove agent |

### 3. Compress (SAFE)

Tables over prose | 1-line rules | Remove examples if obvious | Merge related | No preambles | Abbreviate

**FORBIDDEN**: Remove workflow steps | Drop verify commands | Merge quality gates | Remove `_Why:` | Remove NON-NEGOTIABLE

### 4. Per-project agent pruning

Keep ONLY agents relevant to what the repo does. Archive the rest to `{repo}/.claude/agents-archive/` (outside discovery).

### 5. Verify

```bash
# Recount → compare with baseline → report table
# Verify NON-NEGOTIABLE preserved: grep 'NON.NEGOZI\|NEVER\|ALWAYS'
# Verify workflow: grep 'validate-task\|planner-create\|plan-db-safe'
```

## Hooks to verify

| Hook | Purpose | Check |
|---|---|---|
| `workflow-enforcer.sh` | CI/PR poll 120s cooldown | `ci-digest.sh`, `pr-threads.sh` also rate-limited |
| `gh-credential-router.sh` | Per-repo git auth | All repos mapped in `get_account_for_dir()` |

## Learning management

- MEMORY.md: <30 lines/project. Only quick-ref (env, auth, gotchas) + DB pointers.
- Knowledge base: `plan-db.sh kb-write <domain> "title" "content" --tags '["tag"]'`
- Link to plan: `UPDATE knowledge_base SET source_ref='Plan-XXX' WHERE ...`
- Session learnings: `session-learnings.jsonl` — review with `session-learnings.sh summary`, truncate stale signals

## Schedule

| Trigger | Scope |
|---|---|
| Claude/Copilot update | Full (all phases, all repos) |
| `/optimize-project --ecosystem` | Full |
| Post-plan learning loop | Check new rules for duplicates |
| Quarterly | Full |
