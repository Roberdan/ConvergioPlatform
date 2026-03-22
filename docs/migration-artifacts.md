# Migration Artifacts — MyConvergio to ConvergioPlatform

Generated: 2026-03-22 | Wave: W4 | Plan: 684

## Already Migrated (W1–W3)

The following were confirmed present in both repos or migrated in earlier waves:

| Source (MyConvergio) | Destination (ConvergioPlatform) | Type | Notes |
|---|---|---|---|
| AgenticManifesto.md | AgenticManifesto.md | doc | Migrated T1-02 |
| CLAUDE.md | CLAUDE.md | doc | Migrated |
| AGENTS.md | AGENTS.md | doc | Migrated |
| CONTRIBUTING.md | CONTRIBUTING.md | doc | Migrated |
| SECURITY.md | SECURITY.md | doc | Migrated |
| .github/copilot-instructions.md | .github/copilot-instructions.md | doc | Migrated |
| .gitignore | .gitignore | config | Migrated |
| .claude/settings.json | .claude/settings.json | config | Migrated |
| scripts/mesh/lib/peers.sh | scripts/mesh/lib/peers.sh | script | Migrated |
| scripts/mesh/mesh-auth-sync.sh | scripts/mesh/mesh-auth-sync.sh | script | Migrated |
| scripts/mesh/mesh-heartbeat.sh | scripts/mesh/mesh-heartbeat.sh | script | Migrated |
| scripts/mesh/mesh-sync-all.sh | scripts/mesh/mesh-sync-all.sh | script | Migrated |
| CHANGELOG.md | CHANGELOG.md | doc | Migrated |
| LICENSE | LICENSE | doc | Migrated |
| README.md | README.md | doc | Migrated |
| .claude/commands/execute-modules/error-handling.md | claude-config/commands/execute-modules/error-handling.md | doc | Present in worktree |
| .claude/commands/planner-modules/knowledge-codification.md | claude-config/commands/planner-modules/knowledge-codification.md | doc | Present in worktree |
| .claude/commands/planner-modules/model-strategy.md | claude-config/commands/planner-modules/model-strategy.md | doc | Present in worktree |
| .claude/commands/planner-modules/parallelization-modes.md | claude-config/commands/planner-modules/parallelization-modes.md | doc | Present in worktree |
| config/models.yaml | claude-config/config/models.yaml | config | Present in worktree |
| config/plan-spec-schema.json | claude-config/config/plan-spec-schema.json | config | Present in worktree |
| config/orchestrator.yaml | claude-config/config/orchestrator.yaml | config | Present in worktree |
| config/notifications.conf | claude-config/config/notifications.conf | config | Present in worktree |
| scripts/lib/plan-db/crud-*.sh (4 files) | claude-config/scripts/lib/plan-db-*.sh | script | Superseded — ConvergioPlatform uses consolidated plan-db-crud.sh, plan-db-read.sh, plan-db-update.sh (different architecture) |
| scripts/lib/plan-db/validate-*.sh (5 files) | claude-config/scripts/lib/plan-db-validate.sh | script | Superseded — consolidated into plan-db-validate.sh |

## Not Migrated — Scope Exclusions

| Source | Reason |
|---|---|
| copilot-agents/ (85 files) | Replaced by transpilers |
| copilot-config/ | Copilot-specific config |
| copilot-instructions.md (root) | Copilot-specific |
| .copilot-tracking/ | Copilot plan tracking |
| .claude/agents-lean.archive/ (61 files) | Historical lean variants; archived |
| scripts/mesh/dashboard_textual/ (26 files) | Legacy Python TUI; superseded by Rust TUI in daemon/ |
| scripts/mesh/dashboard_web/ (48 files) | Legacy Python web dashboard; superseded by dashboard/ |
| scripts/dashboard_web/ | Duplicate of above |
| scripts/myconvergio*.sh (5 files) | MyConvergio-specific backup/restore/doctor |
| scripts/generate-copilot-agents.sh | Copilot generation tooling |
| scripts/generate-lean-variants.sh | Lean agent generation |
| systemd/ | Linux systemd units for MyConvergio nightly scheduler |
| tests/bats/ (10 files) | Tests for MyConvergio-specific scripts |
| .claude/scripts/archive/migrations/ (13 files) | Historical DB migrations; already applied |
| .codegraph/ | Machine-specific local index |
| docs/pdf-local-only/ (6 PDFs) | Local-only reference; not tracked in git |
| config/myconvergio-nightly.conf.example | MyConvergio nightly scheduler |
| config/mesh-heartbeat.plist.template | macOS launchd; machine-specific |
| config/mesh-heartbeat.service.template | Linux systemd; machine-specific |
| config/repos.conf | MyConvergio-specific repo list |
| scripts/lib/sync-to-myconvergio-ops.sh | MyConvergio-specific sync logic |
| .claude-snapshot-baseline.json | Snapshot tool state; machine-specific |
| mcp-config.json | Local MCP config; machine-specific |
| install.sh / Makefile | MyConvergio installer |
| VERSION | MyConvergio version tracker |
| CovergioLogoTransparent.webp | Logo asset; not in scope |
| .github/workflows/*.yml (9 files) | MyConvergio-specific CI (copilot sync, lean gen, nightly); not applicable |
| config/peers.conf.example | Machine-specific mesh config |

## Not Migrated — Deferred to Future Plans

These artifacts exist only in MyConvergio and have value for ConvergioPlatform but were out of scope for W4 (task focused on operational scripts, user-facing docs, test strategy docs):

| Source | Destination (proposed) | Type | Notes |
|---|---|---|---|
| hooks/ (31 files) | claude-config/hooks/ | script | All Claude tool hooks (enforce-plan-db-safe, secret-scanner, worktree-guard, session-*, etc.); highest value, separate plan needed |
| hooks.json | claude-config/ | config | Wires hooks to Claude tool events |
| .github/agents/ 76 remaining agents | .github/agents/ | doc | 14/90 agents migrated; business_ops, compliance, design, leadership, release, research, specialized_experts, technical_dev categories pending |
| .github/agents/night-maintenance.agent.md | .github/agents/ | doc | Nightly ops agent |
| .github/CODEOWNERS | .github/ | config | Code ownership definitions |
| .github/ISSUE_TEMPLATE/bug_report.md | .github/ISSUE_TEMPLATE/ | doc | Bug report template |
| scripts/linting/validate-frontmatter.sh | claude-config/scripts/linting/ | script | Frontmatter validation CLI |
| scripts/linting/schemas/ (4 files) | claude-config/scripts/linting/schemas/ | config | agent-frontmatter, copilot-agent-frontmatter, rule-frontmatter, schema-mapping schemas (skill-frontmatter.schema.json already in config/) |
| docs/adr/0001-0020 (20 ADRs) | docs/adr/ | doc | MyConvergio-specific ADRs; evaluate relevance before migrating |
| docs/AGENT_ORCHESTRATION_ARCHITECTURE.md | docs/ | doc | Agent orchestration architecture reference |
| docs/agents/*.md (5 files) | docs/agents/ | doc | Agent portfolio, showcase, architecture, comparison, orchestrator |
| docs/MIGRATION-v10-to-v11.md | docs/ | doc | Version migration guide |
| docs/VERSIONING_POLICY.md | docs/ | doc | Versioning policy |

## Summary

| Category | Count | Status |
|---|---|---|
| Migrated (W1–W3) | 21 | Done |
| Excluded (Copilot/legacy/machine-specific) | 28 groups | Not applicable |
| Deferred (hooks, agents, linting, docs) | ~140 files | Future plan |

The most impactful deferred items are the **31 hook files** (enforce-plan-db-safe, secret-scanner, worktree-guard, session management) and the **76 remaining agent files**. These should be migrated as a dedicated plan once the W4 platform consolidation is complete.
