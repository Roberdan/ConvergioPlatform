# ADR 0103: Virtual Organization Model

Status: Accepted | Date: 21 Mar 2026

## Context

ConvergioPlatform has 84+ specialized AI agents (imported from MyConvergio) covering every business function: engineering, design, strategy, compliance, marketing, HR, finance, operations. The daemon has a complete IPC engine with messaging, delegation, and mesh networking. Missing: a way to dynamically assemble agents into teams based on a high-level goal.

## Decision

1. **Agent Catalog in DB** — `agent_catalog` table stores all agents with name, category, description, model. `agent_skills` maps agents to skills with confidence scores. Ali queries these to find specialists.
2. **Ali Orchestrator** — a meta-agent (Opus) that receives a problem, decomposes it into roles, queries the catalog, assembles a team, creates a plan, dispatches agents, monitors, and reports.
3. **`convergio solve`** — single entry point: user describes a problem, Ali handles everything.
4. **Model/tool agnostic** — agents can run on Claude Code, Copilot CLI, OpenCode, or local LLMs. Tool selected per agent based on cost, capability, and availability.
5. **MyConvergio consolidated** — all 84 agents, configs (models.yaml, orchestrator.yaml), and operational knowledge migrated to ConvergioPlatform. MyConvergio becomes legacy.

## Consequences

- Positive: one command to solve any problem; full agent catalog available; cost-optimized routing
- Negative: requires Opus for orchestration (most expensive model); quality depends on agent prompt quality

## Enforcement

- Entry: `convergio solve "problem"` or `convergio ali`
- Catalog: `sqlite3 $DASHBOARD_DB "SELECT * FROM agent_catalog;"`
- Skills: `sqlite3 $DASHBOARD_DB "SELECT * FROM agent_skills;"`
