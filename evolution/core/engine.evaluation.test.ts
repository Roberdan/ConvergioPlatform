import { describe, it, expect } from 'vitest';
import { EvolutionEngine } from './engine.js';
import type { PlatformAdapter } from './types/adapter.js';
import type { EvaluationResult, Metric } from './types/index.js';
import { BaseEvaluator } from './evaluators/base-evaluator.js';

class RuntimeEvaluator extends BaseEvaluator {
  readonly domain = 'runtime';
  readonly metricFamilies = ['Runtime'] as const;

  protected async analyze(metrics: Metric[]): Promise<Partial<EvaluationResult>> {
    const p95 = metrics.find((metric) => metric.name === 'http.p95_latency_ms');
    if (!p95 || p95.value <= 500) {
      return { anomalies: [], opportunities: [] };
    }

    return {
      anomalies: [
        { metric: 'http.p95_latency_ms', severity: 'high', detail: 'Latency exceeded 500ms' },
      ],
      opportunities: [
        {
          title: 'Enable edge caching',
          description: 'Reduce p95 latency by serving hot content from edge caches',
          estimatedGain: '-20% p95 latency',
          domain: 'runtime',
          suggestedBlastRadius: 'SingleRepo',
        },
      ],
    };
  }
}

const adapter: PlatformAdapter = {
  name: 'test',
  async collectMetrics() {
    return [
      {
        name: 'http.p95_latency_ms',
        value: 620,
        timestamp: Date.now(),
        labels: {},
        family: 'Runtime',
      },
    ];
  },
  async runCanary() {
    return {
      confidence: 0.5,
      pValue: 0.5,
      recommendation: 'Inconclusive',
      delta: 0,
      sideEffects: [],
    };
  },
  async openPR() {
    return { prUrl: 'https://example/pr/1', prNumber: 1 };
  },
  async rollback() {},
  async healthCheck() {
    return { healthy: true, details: 'ok' };
  },
};

describe('EvolutionEngine evaluate integration', () => {
  it('uses registered evaluators and emits composite score audit', async () => {
    const engine = new EvolutionEngine({
      adapters: [adapter],
      evaluators: [new RuntimeEvaluator()],
    });

    const audits: string[] = [];
    engine.onAudit((entry) => audits.push(entry.action));

    const summary = await engine.run();

    expect(summary.evaluations[0]?.domain).toBe('runtime');
    expect(summary.evaluations[0]?.anomalies.length).toBe(1);
    expect(audits).toContain('evaluations.composite');
  });
});
