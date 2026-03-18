import { describe, it, expect } from 'vitest';
import type { Metric, EvaluationResult } from '../types/index.js';
import type { AggregatedPoint } from '../../telemetry/aggregation.js';
import { BaseEvaluator } from './base-evaluator.js';

class TestEvaluator extends BaseEvaluator {
  readonly domain = 'test';
  readonly metricFamilies = ['Runtime'] as const;

  protected async analyze(): Promise<Partial<EvaluationResult>> {
    return {
      anomalies: [
        { metric: 'a', severity: 'low', detail: 'low' },
        { metric: 'b', severity: 'medium', detail: 'medium' },
      ],
      opportunities: [],
    };
  }

  public runDetect(current: number, baseline: number, threshold: number) {
    return this.detectAnomalies(current, baseline, threshold, 'metric');
  }

  public runScore(anomalies: Array<{ metric: string; severity: 'low' | 'medium' | 'high'; detail: string }>) {
    return this.scoreFromAnomalies(anomalies);
  }
}

describe('BaseEvaluator', () => {
  it('detects anomaly with severity derived from ratio delta', () => {
    const evaluator = new TestEvaluator();
    const anomaly = evaluator.runDetect(180, 100, 0.25);
    expect(anomaly).not.toBeNull();
    expect(anomaly?.severity).toBe('high');
  });

  it('computes score penalties from anomalies', () => {
    const evaluator = new TestEvaluator();
    const score = evaluator.runScore([
      { metric: 'a', severity: 'high', detail: 'h' },
      { metric: 'b', severity: 'medium', detail: 'm' },
      { metric: 'c', severity: 'low', detail: 'l' },
    ]);
    expect(score).toBeLessThan(100);
    expect(score).toBeGreaterThanOrEqual(0);
  });

  it('returns evaluated domain result with computed score', async () => {
    const evaluator = new TestEvaluator();
    const metrics: Metric[] = [];
    const history: AggregatedPoint[] = [];
    const result = await evaluator.evaluate(metrics, history);

    expect(result.domain).toBe('test');
    expect(result.anomalies.length).toBe(2);
    expect(result.score).toBe(80);
  });
});
