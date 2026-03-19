import type { AggregatedPoint } from '../../telemetry/aggregation.js';
import type { EvaluationResult, Metric, MetricFamily } from '../types/index.js';

export interface Evaluator {
  readonly domain: string;
  readonly metricFamilies: readonly MetricFamily[];
  evaluate(metrics: Metric[], history: AggregatedPoint[]): Promise<EvaluationResult>;
}
