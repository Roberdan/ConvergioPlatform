import { describe, it, expect } from 'vitest';
import { mkdtempSync, rmSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { spawnSync } from 'child_process';
import { AgentProfiler } from './profiler.js';
import { AgentProposalGenerator } from './agent-proposal-generator.js';

function sqlite(db: string, sql: string): void {
  const result = spawnSync('sqlite3', [db, sql], { encoding: 'utf8' });
  if (result.status !== 0) throw new Error(result.stderr || 'sqlite3 failed');
}

describe('AgentProfiler', () => {
  it('computes profile aggregates', () => {
    const dir = mkdtempSync(join(tmpdir(), 'agent-profiler-'));
    const dbPath = join(dir, 'dashboard.db');
    sqlite(
      dbPath,
      "CREATE TABLE token_usage (agent TEXT, model TEXT, input_tokens INT, output_tokens INT, cost_usd REAL, task_id TEXT, created_at TEXT);" +
        "CREATE TABLE tasks (id TEXT, assignee TEXT, status TEXT, completed_at TEXT, created_at TEXT);" +
        "INSERT INTO token_usage VALUES ('nasra','claude-opus-4.5',40000,20000,0.9,'1',datetime('now'));" +
        "INSERT INTO token_usage VALUES ('nasra','claude-opus-4.5',30000,10000,0.6,'2',datetime('now'));" +
        "INSERT INTO tasks VALUES ('1','nasra','done',datetime('now'),datetime('now'));" +
        "INSERT INTO tasks VALUES ('2','nasra','done',datetime('now'),datetime('now'));" +
        "INSERT INTO tasks VALUES ('3','nasra','in_progress',NULL,datetime('now'));",
    );
    const profile = new AgentProfiler({ dbPath }).profile('nasra', 30);
    expect(profile.totalTasks).toBe(3);
    expect(profile.costPerTask).toBeCloseTo(0.5, 5);
    expect(profile.topModel).toBe('claude-opus-4.5');
    rmSync(dir, { recursive: true, force: true });
  });
});

describe('AgentProposalGenerator', () => {
  it('emits rule-based proposals', () => {
    const proposals = new AgentProposalGenerator().generate({
      agentName: 'nasra',
      windowDays: 30,
      avgTokensPerTask: 60000,
      costPerTask: 0.75,
      completionRate: 0.62,
      topModel: 'claude-opus-4.5',
      totalTasks: 12,
    });
    expect(proposals).toHaveLength(4);
    expect(proposals.every((proposal) => proposal.blastRadius === 'SingleRepo')).toBe(true);
  });
});
