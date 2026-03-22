# MyConvergio vs ConvergioPlatform Audit

Generated: 2026-03-22 | MyConvergio: 1335 files | ConvergioPlatform: 1417 files

## Already Migrated (W1–W3)

| MyConvergio Path | ConvergioPlatform Path | Status |
|---|---|---|
| AgenticManifesto.md | AgenticManifesto.md | Migrated (T1-02) |
| CLAUDE.md | CLAUDE.md | Migrated |
| AGENTS.md | AGENTS.md | Migrated |
| CONTRIBUTING.md | CONTRIBUTING.md | Migrated |
| SECURITY.md | SECURITY.md | Migrated |
| .github/copilot-instructions.md | .github/copilot-instructions.md | Migrated |
| .gitignore | .gitignore | Migrated |
| .claude/settings.json | .claude/settings.json | Migrated |
| scripts/mesh/lib/peers.sh | scripts/mesh/lib/peers.sh | Migrated |
| scripts/mesh/mesh-auth-sync.sh | scripts/mesh/mesh-auth-sync.sh | Migrated |
| scripts/mesh/mesh-heartbeat.sh | scripts/mesh/mesh-heartbeat.sh | Migrated |
| scripts/mesh/mesh-sync-all.sh | scripts/mesh/mesh-sync-all.sh | Migrated |
| CHANGELOG.md | CHANGELOG.md | Migrated |
| LICENSE | LICENSE | Migrated |
| README.md | README.md | Migrated |

## Still Only in MyConvergio

### Hooks (31 files) — Claude tool hooks

| Path | Action |
|---|---|
| hooks/enforce-plan-db-safe.sh | Migrate to claude-config/hooks/ |
| hooks/enforce-plan-edit.sh | Migrate to claude-config/hooks/ |
| hooks/enforce-standards.sh | Migrate to claude-config/hooks/ |
| hooks/env-vault-guard.sh | Migrate to claude-config/hooks/ |
| hooks/guard-plan-mode.sh | Migrate to claude-config/hooks/ |
| hooks/guard-settings.sh | Migrate to claude-config/hooks/ |
| hooks/inject-agent-context.sh | Migrate to claude-config/hooks/ |
| hooks/prefer-ci-summary.sh | Migrate to claude-config/hooks/ |
| hooks/preserve-context.sh | Migrate to claude-config/hooks/ |
| hooks/secret-scanner.sh | Migrate to claude-config/hooks/ |
| hooks/session-file-lock.sh | Migrate to claude-config/hooks/ |
| hooks/session-file-unlock.sh | Migrate to claude-config/hooks/ |
| hooks/session-reaper.sh | Migrate to claude-config/hooks/ |
| hooks/session-task-recovery.sh | Migrate to claude-config/hooks/ |
| hooks/session-tokens.sh | Migrate to claude-config/hooks/ |
| hooks/track-tokens.sh | Migrate to claude-config/hooks/ |
| hooks/verify-before-claim.sh | Migrate to claude-config/hooks/ |
| hooks/warn-bash-antipatterns.sh | Migrate to claude-config/hooks/ |
| hooks/warn-infra-plan-drift.sh | Migrate to claude-config/hooks/ |
| hooks/worktree-guard.sh | Migrate to claude-config/hooks/ |
| hooks/worktree-setup.sh | Migrate to claude-config/hooks/ |
| hooks/worktree-teardown.sh | Migrate to claude-config/hooks/ |
| hooks/enforce-line-limit.sh | Migrate to claude-config/hooks/ |
| hooks/model-registry-refresh.sh | Migrate to claude-config/hooks/ |
| hooks/version-check.sh | Migrate to claude-config/hooks/ |
| hooks/advisory-suggest-alternatives.sh | Migrate to claude-config/hooks/ |
| hooks/auto-format.sh | Migrate to claude-config/hooks/ |
| hooks/debug-hook-input.sh | Migrate — dev utility |
| hooks/lib/common.sh | Migrate to claude-config/hooks/lib/ |
| hooks/lib/file-lock-common.sh | Migrate to claude-config/hooks/lib/ |
| hooks/README.md | Migrate documentation |

### hooks.json — Hook configuration

| Path | Action |
|---|---|
| hooks.json | Migrate — wires hooks to Claude tool events |

### Agents (.claude/agents/ — 90 files, categorical)

| Category | Count | Action |
|---|---|---|
| business_operations (andrea, anna, dave, davide, enrico, fabio, luke, marcello, oliver, sofia, steve + workflows) | 12 | Migrate to .github/agents/ |
| compliance_legal (elena, guardian, luca, sophia, dr-enzo) | 5 | Migrate to .github/agents/ |
| core_utility (thor, strategic-planner-*, CONSTITUTION, wanda, xavier, sentinel, plan-reviewer, task-executor, task-executor-tdd, ...) | ~20 | Partially migrated; evaluate remaining |
| design_ux (jony, sara, stefano) | 3 | Migrate to .github/agents/ |
| leadership_strategy (ali, amy, antonio, dan, domik, matteo, satya) | 7 | Migrate to .github/agents/ |
| release_management (app-release-manager*, ecosystem-sync, feature-release-manager, mirrorbuddy) | 5 | Migrate to .github/agents/ |
| research_report (generator + templates) | 4 | Migrate to .github/agents/ |
| specialized_experts (angela, ava, behice, coach, ethan, evan, fiona, giulia, jenny, michael, riccardo, sam, wiz) | 13 | Migrate to .github/agents/ |
| technical_development (adversarial-debugger, baccio, dario, marco, omri, otto, paolo, rex + references) | 10 | Migrate to .github/agents/ |

### Rules (.claude/rules/ — 16 files)

| Path | Status |
|---|---|
| .claude/rules/ | Most rules already exist in claude-config/rules/; compare for delta content |

### Commands (.claude/commands/ — 12 files + commands/ 3 files)

| Path | Action |
|---|---|
| .claude/commands/execute-modules/error-handling.md | Migrate to claude-config/commands/ |
| .claude/commands/planner-modules/knowledge-codification.md | Migrate to claude-config/commands/ |
| .claude/commands/planner-modules/model-strategy.md | Migrate to claude-config/commands/ |
| .claude/commands/planner-modules/parallelization-modes.md | Migrate to claude-config/commands/ |
| commands/plan.md | Migrate or deprecate |
| commands/status.md | Migrate or deprecate |
| commands/team.md | Migrate or deprecate |

### Scripts — lib only in MyConvergio

| Path | Action |
|---|---|
| scripts/lib/plan-db/crud-*.sh (4) | Migrate — refactored plan-db-core modules |
| scripts/lib/plan-db/validate-*.sh (5) | Migrate — validation gate modules |
| scripts/lib/sync-to-myconvergio-ops.sh | Evaluate — MyConvergio-specific |
| scripts/linting/validate-frontmatter.sh | Migrate to claude-config/ |
| scripts/linting/schemas/*.json (5) | Migrate to claude-config/ |

### ADRs and Docs

| Path | Action |
|---|---|
| docs/adr/0001-0025 (25 ADRs) | Migrate decision records to ConvergioPlatform docs/adr/ |
| docs/adr/ADR-001-ADR-013 (13 ADRs) | Migrate |
| docs/adr/INDEX.md | Migrate |
| docs/AGENT_ORCHESTRATION_ARCHITECTURE.md | Migrate |
| docs/agents/*.md (5) | Migrate |
| docs/MIGRATION-v10-to-v11.md | Migrate |
| docs/VERSIONING_POLICY.md | Migrate |

### .github

| Path | Action |
|---|---|
| .github/agents/night-maintenance.agent.md | Migrate to .github/agents/ |
| .github/CODEOWNERS | Migrate |
| .github/ISSUE_TEMPLATE/bug_report.md | Migrate |
| .github/workflows/*.yml (9 files) | Evaluate — CI workflows for MyConvergio-specific ops |

### Config

| Path | Action |
|---|---|
| config/plan-spec-schema.json | Migrate — shared schema |
| config/models.yaml | Migrate — model catalog |
| config/notifications.conf | Migrate |
| config/orchestrator.yaml | Migrate |
| config/peers.conf.example | Migrate |
| config/repos.conf | Evaluate — MyConvergio-specific |

## MyConvergio-Specific (Stay in MyConvergio)

| Path | Reason |
|---|---|
| copilot-agents/ (85 files) | Copilot-only agent format; being replaced by transpilers |
| copilot-config/ | Copilot-specific config |
| copilot-instructions.md | Copilot root-level instruction |
| .copilot-tracking/ | Copilot plan tracking |
| .claude/agents-lean.archive/ (61 files) | Archived lean variants; historical only |
| scripts/mesh/dashboard_textual/ (26 files) | Legacy Python TUI; superseded by Rust TUI |
| scripts/mesh/dashboard_web/ (48 files) | Legacy Python web dashboard; superseded by dashboard/ |
| scripts/dashboard_web/ (top-level) | Duplicate of above |
| scripts/myconvergio*.sh (5 files) | Backup/restore/doctor for MyConvergio itself |
| scripts/generate-copilot-agents.sh | Copilot generation tooling |
| scripts/generate-lean-variants.sh | Lean agent generation |
| systemd/ | Linux systemd units for MyConvergio nightly |
| tests/bats/ (10 files) | Tests for MyConvergio-specific scripts |
| .claude/scripts/archive/migrations/ (13 files) | Historical DB migrations; already applied |
| .claude/scripts/dashboard_textual/ | Legacy TUI |
| .claude/scripts/dashboard_web/ | Legacy Python web |
| .codegraph/ | Local codegraph DB; machine-specific |
| docs/pdf-local-only/ (6 PDFs) | Local-only reference docs |
| config/myconvergio-nightly.conf.example | MyConvergio nightly scheduler |
| config/mesh-heartbeat.plist.template | macOS launchd config |
| config/mesh-heartbeat.service.template | Linux systemd config |
| .claude-snapshot-baseline.json | Snapshot tool state |
| .claude-plugin/plugin.json | Plugin config |
| mcp-config.json | Local MCP config |
| install.sh / Makefile | MyConvergio installer |
| VERSION | MyConvergio version tracker |
| CovergioLogoTransparent.webp | Logo asset |

## Summary

| Metric | Count |
|---|---|
| Total MyConvergio files | 1335 |
| Already migrated (common to both) | 15 |
| To migrate (hooks, agents, commands, scripts/lib, ADRs, config) | ~250 |
| Stay in MyConvergio (Copilot, legacy dashboards, backups, systemd) | ~1070 |
