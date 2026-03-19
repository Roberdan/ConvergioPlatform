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
    const memoryMb = latestValue(metrics, 'runtime.memory_mb');
    const cpuPct = latestValue(metrics, 'runtime.cpu_pct');

    if (queueDepth !== null && queueDepth > 500) {
      anomalies.push({ metric: 'workload.queue_depth', severity: 'high', detail: `Queue depth=${queueDepth} > 500` });
    } else if (queueDepth !== null && queueDepth > 100) {
      anomalies.push({ metric: 'workload.queue_depth', severity: 'medium', detail: `Queue depth=${queueDepth} > 100` });
    }

    const errorPct = errorRate !== null && errorRate <= 1 ? errorRate * 100 : errorRate;
    if (errorPct !== null && errorPct > 5) {
      anomalies.push({ metric: 'workload.task_error_rate', severity: 'medium', detail: `Error rate=${errorPct.toFixed(1)}% > 5%` });
    }

    if (memoryMb !== null && memoryMb > 8000) {
      anomalies.push({ metric: 'runtime.memory_mb', severity: 'medium', detail: `Memory=${memoryMb}MB > 8000MB` });
    }

    if (cpuPct !== null && cpuPct > 90) {
      anomalies.push({ metric: 'runtime.cpu_pct', severity: 'high', detail: `CPU=${cpuPct}% > 90%` });
    }

    return {
      anomalies,
      opportunities: anomalies.length
        ? opportunities(this.domain, ['Scale worker pool', 'Add circuit breaker', 'Implement backpressure', 'Optimize memory usage'], '-20% queue and error pressure')
        : [],
    };
  }
}
