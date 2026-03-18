import { describe, it, expect } from 'vitest';
import { mkdtempSync, rmSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { spawnSync } from 'child_process';
import { ModelIntelligence } from './model-registry.js';
import { BenchmarkRunner } from './benchmark-runner.js';

function sqlite(db: string, sql: string): void {
  const result = spawnSync('sqlite3', [db, sql], { encoding: 'utf8' });
  if (result.status !== 0) throw new Error(result.stderr || 'sqlite3 failed');
}

describe('ModelIntelligence', () => {
  it('detects upgrade opportunities from registry', () => {
    const dir = mkdtempSync(join(tmpdir(), 'model-intel-'));
    const dbPath = join(dir, 'dashboard.db');
    sqlite(
      dbPath,
      "CREATE TABLE ipc_model_registry (id TEXT, model_id TEXT, version TEXT, cost_per_1k REAL);" +
        "INSERT INTO ipc_model_registry VALUES ('1','claude-opus','4.5',0.02);",
    );

    const intelligence = new ModelIntelligence({ dbPath });
    const upgrades = intelligence.checkForUpgrades();
    expect(upgrades).toHaveLength(1);
    expect(upgrades[0]?.latestVersion).toBe('4.6');
    rmSync(dir, { recursive: true, force: true });
  });
});

describe('BenchmarkRunner', () => {
  it('computes benchmark summary for model/task type', () => {
    const dir = mkdtempSync(join(tmpdir(), 'benchmark-'));
    const dbPath = join(dir, 'dashboard.db');
    sqlite(
      dbPath,
      "CREATE TABLE tasks (id TEXT, status TEXT, task_type TEXT, created_at TEXT, started_at TEXT, completed_at TEXT);" +
        "CREATE TABLE token_usage (task_id TEXT, model TEXT, input_tokens INT, output_tokens INT, cost_usd REAL);" +
        "INSERT INTO tasks VALUES ('1','done','analysis',datetime('now','-20 min'),datetime('now','-10 min'),datetime('now'));" +
        "INSERT INTO token_usage VALUES ('1','gpt-5',1000,500,0.2);",
    );

    const result = new BenchmarkRunner({ dbPath }).run('gpt-5', 'analysis');
    expect(result.sampleSize).toBe(1);
    expect(result.avgTokens).toBe(1500);
    rmSync(dir, { recursive: true, force: true });
  });
});
