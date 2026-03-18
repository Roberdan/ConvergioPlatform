import { describe, expect, it, vi } from 'vitest';
import type { Metric } from '../../../evolution/core/types/index.ts';
import { ReplayRunner } from '../../../evolution/experiments/replay.ts';
import { RollbackManager } from '../../../evolution/experiments/rollback.ts';
import { ReportGenerator } from '../../../evolution/experiments/report-generator.ts';

const now = Date.now();
const metrics: Metric[] = [
  { name: 'http.p95_latency_ms', value: 200, timestamp: now - 10_000, labels: {}, family: 'Runtime' },
  { name: 'http.p95_latency_ms', value: 170, timestamp: now - 1_000, labels: {}, family: 'Runtime' },
];

describe('ReplayRunner', () => {
  it('replays historical metrics and produces what-if ExperimentResult', async () => {
    const store = {
      query: vi.fn(() => metrics),
    };

    const replay = new ReplayRunner();
    const result = await replay.replay('exp-123', store);

    expect(store.query).toHaveBeenCalledOnce();
    expect(result).toHaveProperty('pValue');
    expect(result.delta).toBeLessThan(0);
  });
});

describe('RollbackManager', () => {
  it('builds revertPR metadata and calls adapter rollback', async () => {
    const rollback = vi.fn(async () => undefined);
    const manager = new RollbackManager();

    const info = await manager.rollback('exp-rollback', { rollback });

    expect(info.revertPR.prUrl).toContain('rollback');
    expect(rollback).toHaveBeenCalledWith('exp-rollback');
  });
});

describe('ReportGenerator', () => {
  it('includes confidenceInterval, pValue and beforeAfter snapshots', () => {
    const markdown = new ReportGenerator().generate({
      experimentId: 'exp-xyz',
      mode: 'Canary',
      proposalTitle: 'Tune cache strategy',
      durationMs: 12_000,
      beforeAfter: {
        before: [{ name: 'latency', value: 210 }],
        after: [{ name: 'latency', value: 170 }],
      },
      confidenceInterval: [-0.23, -0.11],
      pValue: 0.02,
      recommendation: 'Apply',
      anomaliesResolved: ['latency spike'],
      deltaScore: -0.19,
    });

    expect(markdown).toContain('confidenceInterval');
    expect(markdown).toContain('pValue');
    expect(markdown).toContain('beforeAfter');
  });
});
