import { describe, it, expect } from 'vitest';
import { mkdtempSync, rmSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { spawnSync } from 'child_process';
import { NaSraCanaryAdapter } from './nasra-canary.js';

function sqlite(db: string, sql: string): void {
  const result = spawnSync('sqlite3', [db, sql], { encoding: 'utf8' });
  if (result.status !== 0) throw new Error(result.stderr || 'sqlite3 failed');
}

describe('NaSraCanaryAdapter', () => {
  it('collects metrics as Agent family', async () => {
    const dir = mkdtempSync(join(tmpdir(), 'nasra-canary-'));
    const dbPath = join(dir, 'dashboard.db');
    sqlite(
      dbPath,
      "CREATE TABLE plan_actuals (metric TEXT, actual_tokens REAL);" +
        "CREATE TABLE collector_runs (status TEXT);" +
        "INSERT INTO plan_actuals VALUES ('bundle_size_kb', 120);" +
        "INSERT INTO collector_runs VALUES ('success');" +
        "INSERT INTO collector_runs VALUES ('failed');",
    );
    const metrics = await new NaSraCanaryAdapter({ dbPath, repoPath: dir }).collectMetrics();
    expect(metrics).toHaveLength(2);
    expect(metrics.every((metric) => metric.family === 'Agent')).toBe(true);
    rmSync(dir, { recursive: true, force: true });
  });
});
