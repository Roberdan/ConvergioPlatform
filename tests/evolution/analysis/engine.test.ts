import { describe, expect, it } from 'vitest';
import { AnalysisEngine } from '../../../evolution/analysis/engine.ts';
import type { Metric } from '../../../evolution/core/types/index.ts';

describe('AnalysisEngine', () => {
  it('returns anomaly and optimization opportunities from metric drift', () => {
    const now = Date.now();
    const metrics: Metric[] = [
      { name: 'http.p95_latency_ms', value: 100, timestamp: now - 2_000, labels: {}, family: 'Runtime' },
      { name: 'http.p95_latency_ms', value: 110, timestamp: now - 1_000, labels: {}, family: 'Runtime' },
      { name: 'http.p95_latency_ms', value: 190, timestamp: now, labels: {}, family: 'Runtime' },
    ];

    const engine = new AnalysisEngine();
    const outcome = engine.analyze(metrics);

    expect(outcome.anomalies.length).toBe(1);
    expect(outcome.opportunities.length).toBe(1);
    expect(outcome.opportunities[0]?.title).toContain('Investigate');
  });
});
