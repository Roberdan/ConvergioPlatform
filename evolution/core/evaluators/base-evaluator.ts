import type { AggregatedPoint } from '../../telemetry/aggregation.js';
import type { EvaluationResult, Metric, OptimizationOpportunity } from '../types/index.js';
import type { Evaluator } from './evaluator.js';

type Anomaly = EvaluationResult['anomalies'][number];

type PartialEvaluation = Partial<Pick<EvaluationResult, 'anomalies' | 'opportunities'>>;

const SEVERITY_WEIGHT: Record<Anomaly['severity'], number> = {
  low: 5,
  medium: 15,
  high: 30,
};

export abstract class BaseEvaluator implements Evaluator {
  abstract readonly domain: string;
  abstract readonly metricFamilies: readonly Metric['family'][];

  async evaluate(metrics: Metric[], history: AggregatedPoint[]): Promise<EvaluationResult> {
    const partial = await this.analyze(metrics, history);
    const anomalies = partial.anomalies ?? [];
    const opportunities: OptimizationOpportunity[] = partial.opportunities ?? [];

    return {
      domain: this.domain,
      anomalies,
      opportunities,
      score: this.scoreFromAnomalies(anomalies),
    };
  }

  protected detectAnomalies(
    current: number,
    baseline: number,
    threshold: number,
    metric: string,
  ): Anomaly | null {
    if (baseline <= 0) {
      return null;
    }

    const deltaRatio = Math.abs(current - baseline) / baseline;
    if (deltaRatio <= threshold) {
      return null;
    }

    const severity: Anomaly['severity'] =
      deltaRatio > 0.5 ? 'high' : deltaRatio > 0.25 ? 'medium' : 'low';

    return {
      metric,
      severity,
      detail: `Current=${current.toFixed(2)}, baseline=${baseline.toFixed(2)}, ratio=${deltaRatio.toFixed(3)}`,
    };
  }

  protected scoreFromAnomalies(anomalies: Anomaly[]): number {
    const penalty = anomalies.reduce((sum, anomaly) => sum + SEVERITY_WEIGHT[anomaly.severity], 0);
    return Math.max(0, 100 - penalty);
  }

  protected abstract analyze(
    metrics: Metric[],
    history: AggregatedPoint[],
  ): Promise<PartialEvaluation>;
}
