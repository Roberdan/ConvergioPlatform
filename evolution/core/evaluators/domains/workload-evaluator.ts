import type { AggregatedPoint } from '../../../telemetry/aggregation.js';
import type { Metric, EvaluationResult } from '../../types/index.js';
import { BaseEvaluator } from '../base-evaluator.js';
import { latestValue, opportunities } from './shared.js';

export class WorkloadEvaluator extends BaseEvaluator {
  readonly domain = 'workload';
  readonly metricFamilies = ['Workload', 'Runtime'] as const;

  protected async analyze(metrics: Metric[], _history: AggregatedPoint[]): Promise<Partial<EvaluationResult>> {
    const anomalies: EvaluationResult['anomalies'] = [];
    const queueDepth = latestValue(metrics, 'workload.queue_depth');
    const errorRate = latestValue(metrics, 'workload.task_error_rate');

    if (queueDepth !== null && queueDepth > 500) {
      anomalies.push({ metric: 'workload.queue_depth', severity: 'high', detail: `Queue depth=${queueDepth} > 500` });
    } else if (queueDepth !== null && queueDepth > 100) {
      anomalies.push({ metric: 'workload.queue_depth', severity: 'medium', detail: `Queue depth=${queueDepth} > 100` });
    }

    if (errorRate !== null && errorRate > 0.15) {
      anomalies.push({ metric: 'workload.task_error_rate', severity: 'high', detail: `Error rate=${(errorRate * 100).toFixed(1)}% > 15%` });
    } else if (errorRate !== null && errorRate > 0.05) {
      anomalies.push({ metric: 'workload.task_error_rate', severity: 'medium', detail: `Error rate=${(errorRate * 100).toFixed(1)}% > 5%` });
    }

    return {
      anomalies,
      opportunities: anomalies.length
        ? opportunities(this.domain, ['Scale worker pool', 'Add circuit breaker', 'Implement backpressure'], '-20% queue and error pressure')
        : [],
    };
  }
}
