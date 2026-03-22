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

## Removed: copilot-agents/

Removed in W4 (T4-06) — 2026-03-22. These 85 Copilot CLI agent wrappers are superseded by the universal skill protocol + transpilers created in W3.

**Rationale**: The transpiler approach generates provider-specific agent formats on demand from canonical `.claude/agents/` source files. Maintaining 85 hand-crafted wrappers in parallel is redundant and creates drift risk.

| File | Category |
|---|---|
| adversarial-debugger.agent.md | technical_development |
| ali-chief-of-staff.agent.md | leadership_strategy |
| amy-cfo.agent.md | leadership_strategy |
| andrea-customer-success-manager.agent.md | business_operations |
| angela-da.agent.md | specialized_experts |
| anna-executive-assistant.agent.md | business_operations |
| antonio-strategy-expert.agent.md | leadership_strategy |
| app-release-manager-execution.agent.md | release_management |
| app-release-manager.agent.md | release_management |
| ava-analytics-insights-virtuoso.agent.md | specialized_experts |
| baccio-tech-architect.agent.md | technical_development |
| behice-cultural-coach.agent.md | specialized_experts |
| check.agent.md | core_utility (workflow wrapper) |
| coach-team-coach.agent.md | specialized_experts |
| code-reviewer.agent.md | technical_development |
| compliance-checker.agent.md | compliance_legal |
| dan-engineering-gm.agent.md | leadership_strategy |
| dario-debugger.agent.md | technical_development |
| dario-debugger.lean.agent.md | technical_development (lean variant) |
| dave-change-management-specialist.agent.md | business_operations |
| davide-project-manager.agent.md | business_operations |
| deep-repo-auditor.agent.md | technical_development |
| diana-performance-dashboard.agent.md | specialized_experts |
| domik-mckinsey-strategic-decision-maker.agent.md | leadership_strategy |
| dr-enzo-healthcare-compliance-manager.agent.md | compliance_legal |
| ecosystem-sync.agent.md | release_management |
| elena-legal-compliance-expert.agent.md | compliance_legal |
| enrico-business-process-engineer.agent.md | business_operations |
| ethan-da.agent.md | specialized_experts |
| evan-ic6da.agent.md | specialized_experts |
| execute.agent.md | core_utility (workflow wrapper) |
| EXECUTION_DISCIPLINE.agent.md | core_utility |
| fabio-sales-business-development.agent.md | business_operations |
| feature-release-manager.agent.md | release_management |
| fiona-market-analyst.agent.md | specialized_experts |
| giulia-hr-talent-acquisition.agent.md | specialized_experts |
| guardian-ai-security-validator.agent.md | compliance_legal |
| jenny-inclusive-accessibility-champion.agent.md | specialized_experts |
| jony-creative-director.agent.md | design_ux |
| knowledge-base.agent.md | core_utility |
| luca-security-expert.agent.md | compliance_legal |
| luke-program-manager.agent.md | business_operations |
| marcello-pm.agent.md | business_operations |
| marco-devops-engineer.agent.md | technical_development |
| marcus-context-memory-keeper.agent.md | core_utility |
| matteo-strategic-business-architect.agent.md | leadership_strategy |
| michael-vc.agent.md | specialized_experts |
| oliver-pm.agent.md | business_operations |
| omri-data-scientist.agent.md | technical_development |
| optimize-project.agent.md | core_utility |
| otto-performance-optimizer.agent.md | technical_development |
| paolo-best-practices-enforcer.agent.md | technical_development |
| plan-business-advisor.agent.md | core_utility |
| plan-post-mortem.agent.md | core_utility |
| plan-reviewer.agent.md | core_utility |
| planner.agent.md | core_utility (workflow wrapper) |
| po-prompt-optimizer.agent.md | core_utility |
| pr-comment-resolver.agent.md | technical_development |
| prompt.agent.md | core_utility (workflow wrapper) |
| research-report-generator.agent.md | research_report |
| rex-code-reviewer.agent.md | technical_development |
| riccardo-storyteller.agent.md | specialized_experts |
| sam-startupper.agent.md | specialized_experts |
| sara-ux-ui-designer.agent.md | design_ux |
| satya-board-of-directors.agent.md | leadership_strategy |
| sentinel-ecosystem-guardian.agent.md | core_utility |
| socrates-first-principles-reasoning.agent.md | core_utility |
| sofia-marketing-strategist.agent.md | business_operations |
| sophia-govaffairs.agent.md | compliance_legal |
| stefano-design-thinking-facilitator.agent.md | design_ux |
| steve-executive-communication-strategist.agent.md | business_operations |
| strategic-planner-git.agent.md | core_utility (planner module) |
| strategic-planner-templates.agent.md | core_utility (planner module) |
| strategic-planner-thor.agent.md | core_utility (planner module) |
| strategic-planner.agent.md | core_utility |
| task-executor-tdd.agent.md | core_utility |
| task-executor.agent.md | core_utility |
| taskmaster-strategic-task-decomposition-master.agent.md | core_utility |
| tdd-executor.agent.md | core_utility |
| thor-quality-assurance-guardian.agent.md | core_utility |
| thor-validation-gates.agent.md | core_utility |
| validate.agent.md | core_utility (workflow wrapper) |
| wanda-workflow-orchestrator.agent.md | core_utility |
| wiz-investor-venture-capital.agent.md | specialized_experts |
| xavier-coordination-patterns.agent.md | core_utility |

**Total removed**: 85 files
