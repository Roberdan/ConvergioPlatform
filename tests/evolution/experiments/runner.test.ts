import { describe, it, expect, vi } from 'vitest';
import type { Experiment, ExperimentResult, Proposal } from '../../../evolution/core/types/index.ts';
import type { PlatformAdapter } from '../../../evolution/core/types/adapter.ts';
import { ExperimentRunner } from '../../../evolution/experiments/runner.ts';

function baseProposal(id: string): Proposal {
  return {
    id,
    hypothesis: 'Improve latency',
    targetMetric: 'runtime.score',
    expectedDelta: { min: -0.2, max: -0.05 },
    successCriteria: 'lower runtime latency',
    failureCriteria: 'higher error rate',
    blastRadius: 'SingleRepo',
    sourceType: 'Internal',
    status: 'Approved',
  };
}

function makeExperiment(mode: Experiment['mode']): Experiment {
  return {
    id: `exp-${mode.toLowerCase()}`,
    proposalId: 'EVO-20260318-0001',
    mode,
    startedAt: Date.now(),
    beforeMetrics: [],
    afterMetrics: [],
  };
}

describe('ExperimentRunner', () => {
  it('runs Shadow mode by comparing before/after metrics without mutation', async () => {
    const rollback = vi.fn(async () => undefined);
    const adapter: PlatformAdapter = {
      name: 'test',
      collectMetrics: async () => [
        { name: 'http.p95_latency_ms', value: 120, timestamp: Date.now(), labels: {}, family: 'Runtime' },
      ],
      runCanary: async () => ({ confidence: 0.9, pValue: 0.03, recommendation: 'Apply', delta: -0.1, sideEffects: [] }),
      openPR: async () => ({ prUrl: 'https://example/pull/1', prNumber: 1 }),
      rollback,
      healthCheck: async () => ({ healthy: true, details: 'ok' }),
    };
    const audit = vi.fn();
    const runner = new ExperimentRunner({ auditSink: audit });

    const result = await runner.run(makeExperiment('Shadow'), adapter, baseProposal('EVO-20260318-0001'));

    expect(result).toHaveProperty('delta');
    expect(rollback).not.toHaveBeenCalled();
    expect(audit).toHaveBeenCalled();
  });

  it('runs Canary mode and rolls back when result is non-apply', async () => {
    const rollback = vi.fn(async () => undefined);
    const canaryResult: ExperimentResult = {
      confidence: 0.35,
      pValue: 0.7,
      recommendation: 'Reject',
      delta: 0.06,
      sideEffects: [{ metric: 'error_rate', delta: 0.02 }],
    };

    const adapter: PlatformAdapter = {
      name: 'test',
      collectMetrics: async () => [],
      runCanary: async () => canaryResult,
      openPR: async () => ({ prUrl: 'https://example/pull/2', prNumber: 2 }),
      rollback,
      healthCheck: async () => ({ healthy: true, details: 'ok' }),
    };

    const runner = new ExperimentRunner({});
    const result = await runner.run(makeExperiment('Canary'), adapter, baseProposal('EVO-20260318-0002'));

    expect(result.recommendation).toBe('Reject');
    expect(rollback).toHaveBeenCalledOnce();
  });

  it('runs Full mode and waits for merge signal before collecting after metrics', async () => {
    const adapter: PlatformAdapter = {
      name: 'test',
      collectMetrics: vi
        .fn<() => Promise<ReturnType<PlatformAdapter['collectMetrics']> extends Promise<infer R> ? R : never>>()
        .mockResolvedValueOnce([
          { name: 'bundle.kb', value: 510, timestamp: Date.now(), labels: {}, family: 'Bundle' },
        ])
        .mockResolvedValueOnce([
          { name: 'bundle.kb', value: 420, timestamp: Date.now(), labels: {}, family: 'Bundle' },
        ]),
      runCanary: async () => ({ confidence: 0.9, pValue: 0.03, recommendation: 'Apply', delta: -0.1, sideEffects: [] }),
      openPR: async () => ({ prUrl: 'https://example/pull/3', prNumber: 3 }),
      rollback: async () => undefined,
      healthCheck: async () => ({ healthy: true, details: 'ok' }),
    };

    const waitForMerge = vi.fn(async () => undefined);
    const runner = new ExperimentRunner({ waitForMerge });
    const result = await runner.run(makeExperiment('Full'), adapter, baseProposal('EVO-20260318-0003'));

    expect(waitForMerge).toHaveBeenCalledWith(3);
    expect(result.recommendation).toBe('Apply');
  });

  it('auto-rolls back on unhandled mode error', async () => {
    const rollback = vi.fn(async () => undefined);
    const adapter: PlatformAdapter = {
      name: 'test',
      collectMetrics: async () => {
        throw new Error('metric collection failed');
      },
      runCanary: async () => ({ confidence: 0.8, pValue: 0.1, recommendation: 'Apply', delta: -0.02, sideEffects: [] }),
      openPR: async () => ({ prUrl: 'https://example/pull/4', prNumber: 4 }),
      rollback,
      healthCheck: async () => ({ healthy: true, details: 'ok' }),
    };

    const runner = new ExperimentRunner({});
    await expect(runner.run(makeExperiment('Shadow'), adapter, baseProposal('EVO-20260318-0004'))).rejects.toThrow();
    expect(rollback).toHaveBeenCalled();
  });
});
