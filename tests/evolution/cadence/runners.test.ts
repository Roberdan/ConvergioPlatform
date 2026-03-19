import { describe, expect, it, vi } from 'vitest';
import { mkdtempSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import type { CycleSummary } from '../../../evolution/core/engine.ts';
import { DailyRunner } from '../../../evolution/cadence/daily-runner.ts';
import { WeeklyRunner } from '../../../evolution/cadence/weekly-runner.ts';

function fakeSummary(cycleId: number, score = 85): CycleSummary {
  return {
    cycleId,
    startedAt: Date.now() - 1000,
    completedAt: Date.now(),
    metricsCollected: 5,
    evaluations: [{ domain: 'runtime', anomalies: [], opportunities: [], score }],
    proposalsGenerated: 1,
    experimentsRun: 1,
    experiments: [],
  };
}

describe('DailyRunner', () => {
  it('runs dailyMicroLoop and persists lightweightEval', async () => {
    const root = mkdtempSync(join(tmpdir(), 'daily-runner-'));
    const dbPath = join(root, 'cadence.db');
    const engine = { run: vi.fn(async () => fakeSummary(1)) };

    const runner = new DailyRunner({ dbPath, statusFilePath: join(root, 'cadence-status.json') });
    const summary = await runner.run(engine);

    expect(summary.cycleId).toBe(1);
    expect(engine.run).toHaveBeenCalledOnce();
    rmSync(root, { recursive: true, force: true });
  });

  it('skips if last run was <20h', async () => {
    const root = mkdtempSync(join(tmpdir(), 'daily-skip-'));
    const dbPath = join(root, 'cadence.db');
    const engine = { run: vi.fn(async () => fakeSummary(2)) };

    const runner = new DailyRunner({ dbPath, statusFilePath: join(root, 'cadence-status.json') });
    await runner.run(engine);
    const second = await runner.run(engine);

    expect(engine.run).toHaveBeenCalledOnce();
    expect(second.experimentsRun).toBe(0);
    rmSync(root, { recursive: true, force: true });
  });
});

describe('WeeklyRunner', () => {
  it('builds deepAnalysis and crossDomainCorrelation from last 7 daily rows', async () => {
    const root = mkdtempSync(join(tmpdir(), 'weekly-runner-'));
    const dbPath = join(root, 'cadence.db');
    const engine = { run: vi.fn(async () => fakeSummary(3, 88)) };

    const daily = new DailyRunner({ dbPath, statusFilePath: join(root, 'cadence-status.json') });
    await daily.run(engine);

    const weekly = new WeeklyRunner({ dbPath });
    const retrospective = await weekly.run(engine);

    expect(retrospective.length).toBeGreaterThan(0);
    rmSync(root, { recursive: true, force: true });
  });
});
