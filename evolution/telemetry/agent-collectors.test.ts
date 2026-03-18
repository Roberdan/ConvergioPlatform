import { describe, it, expect } from 'vitest';
import { mkdtempSync } from 'fs';
import { rmSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { spawnSync } from 'child_process';
import { AgentMetricCollector } from './agent-collectors.js';

function sqlite(db: string, sql: string): void {
  const result = spawnSync('sqlite3', [db, sql], { encoding: 'utf8' });
  if (result.status !== 0) {
    throw new Error(result.stderr || 'sqlite3 failed');
  }
}

describe('AgentMetricCollector', () => {
  it('collects tokenUsage, costPerTask, completionRate and modelSelection metrics', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'agent-collector-'));
    const dbPath = join(dir, 'dashboard.db');

    sqlite(
      dbPath,
      "CREATE TABLE token_usage (input_tokens INT, output_tokens INT, cost_usd REAL, model TEXT, created_at TEXT);" +
        "CREATE TABLE tasks (status TEXT);" +
        "CREATE TABLE plans (status TEXT);" +
        "INSERT INTO token_usage VALUES (100,50,0.12,'gpt-5',datetime('now'));" +
        "INSERT INTO token_usage VALUES (20,10,0.03,'claude-opus-4.6',datetime('now'));" +
        "INSERT INTO tasks VALUES ('done');" +
        "INSERT INTO tasks VALUES ('in_progress');" +
        "INSERT INTO plans VALUES ('doing');" +
        "INSERT INTO plans VALUES ('todo');",
    );

    const collector = new AgentMetricCollector({ dbPath, windowMs: 60 * 60 * 1000 });
    const metrics = await collector.collect();

    const names = new Set(metrics.map((metric) => metric.name));
    expect(names.has('agent.tokens.input')).toBe(true);
    expect(names.has('agent.tokens.output')).toBe(true);
    expect(names.has('agent.cost.usd')).toBe(true);
    expect(names.has('agent.task.completion_rate')).toBe(true);
    expect(names.has('agent.session.count')).toBe(true);
    expect(names.has('agent.model.top')).toBe(true);
    expect(names.has('agent.plan.active_count')).toBe(true);

    const topModel = metrics.find((metric) => metric.name === 'agent.model.top');
    expect(topModel?.labels.model).toBe('gpt-5');

    rmSync(dir, { recursive: true, force: true });
  });
});
