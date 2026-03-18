import type { AggregatedPoint } from '../../../telemetry/aggregation.js';
import type { Metric, EvaluationResult } from '../../types/index.js';
import { BaseEvaluator } from '../base-evaluator.js';
import { latestValue, opportunities } from './shared.js';

export class LatencyEvaluator extends BaseEvaluator {
  readonly domain = 'latency';
  readonly metricFamilies = ['Runtime', 'Workload'] as const;

  protected async analyze(metrics: Metric[], _history: AggregatedPoint[]): Promise<Partial<EvaluationResult>> {
    const anomalies: EvaluationResult['anomalies'] = [];
    const p95 = latestValue(metrics, 'http.p95_latency_ms');
    const p50 = latestValue(metrics, 'http.p50_latency_ms');
    const errorRate = latestValue(metrics, 'http.error_rate_pct');

    if (p95 !== null && p95 > 500) {
      anomalies.push({ metric: 'http.p95_latency_ms', severity: 'high', detail: `p95=${p95}ms > 500ms` });
    }
    if (p50 !== null && p50 > 200) {
      anomalies.push({ metric: 'http.p50_latency_ms', severity: 'medium', detail: `p50=${p50}ms > 200ms` });
    }
    if (errorRate !== null && errorRate > 2) {
      anomalies.push({ metric: 'http.error_rate_pct', severity: 'medium', detail: `Error rate=${errorRate}% > 2%` });
    }

    return {
      anomalies,
      opportunities: anomalies.length
        ? opportunities(this.domain, ['Enable HTTP/2', 'Add edge caching', 'Optimize database queries'], '-10% to -25% latency')
        : [],
    };
  }
}
