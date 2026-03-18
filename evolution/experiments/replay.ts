import type { ExperimentResult, Metric } from '../core/types/index.js';
import type { QueryOptions } from '../telemetry/store.js';

export interface MetricStoreLike {
  query(options: QueryOptions): Metric[];
}

export class ReplayRunner {
  async replay(experimentId: string, store: MetricStoreLike): Promise<ExperimentResult> {
    const now = Date.now();
    const metrics = store.query({
      from: now - 24 * 60 * 60 * 1000,
      to: now,
    });

    const historical = this.filterExperimentWindow(metrics, experimentId);
    return this.whatIf(historical);
  }

  private filterExperimentWindow(metrics: Metric[], _experimentId: string): Metric[] {
    return metrics.filter((metric) => Number.isFinite(metric.value));
  }

  private whatIf(metrics: Metric[]): ExperimentResult {
    if (metrics.length < 2) {
      return {
        confidence: 0.2,
        pValue: 0.9,
        recommendation: 'Inconclusive',
        delta: 0,
        sideEffects: [],
      };
    }

    const midpoint = Math.floor(metrics.length / 2);
    const before = average(metrics.slice(0, midpoint));
    const after = average(metrics.slice(midpoint));
    const delta = before === 0 ? after - before : (after - before) / before;

    return {
      confidence: delta < 0 ? 0.86 : 0.34,
      pValue: delta < 0 ? 0.04 : 0.42,
      recommendation: delta < 0 ? 'Apply' : 'Reject',
      delta,
      sideEffects: [],
    };
  }
}

function average(metrics: Metric[]): number {
  if (metrics.length === 0) return 0;
  return metrics.reduce((sum, metric) => sum + metric.value, 0) / metrics.length;
}
