import type { AggregatedPoint } from '../../../telemetry/aggregation.js';
import type { Metric, EvaluationResult } from '../../types/index.js';
import { BaseEvaluator } from '../base-evaluator.js';
import { latestValue, opportunities } from './shared.js';

export class AgentCostEvaluator extends BaseEvaluator {
  readonly domain = 'agent_cost';
  readonly metricFamilies = ['Agent'] as const;

  protected async analyze(metrics: Metric[], history: AggregatedPoint[]): Promise<Partial<EvaluationResult>> {
    const anomalies: EvaluationResult['anomalies'] = [];
    const currentCost = latestValue(metrics, 'agent.cost.usd');
    const completionRate = latestValue(metrics, 'agent.task.completion_rate');

    const baselineRows = history.filter((point) => point.name === 'agent.cost.usd');
    const baseline = baselineRows.length
      ? baselineRows.reduce((sum, point) => sum + point.avg, 0) / baselineRows.length
      : currentCost;

    if (currentCost !== null && baseline && baseline > 0) {
      const spike = (currentCost - baseline) / baseline;
      if (spike > 0.5) {
        anomalies.push({ metric: 'agent.cost.usd', severity: 'high', detail: `Cost spike ${(spike * 100).toFixed(1)}% vs baseline` });
      } else if (spike > 0.25) {
        anomalies.push({ metric: 'agent.cost.usd', severity: 'medium', detail: `Cost spike ${(spike * 100).toFixed(1)}% vs baseline` });
      }
    }

    if (completionRate !== null && completionRate < 0.4) {
      anomalies.push({ metric: 'agent.task.completion_rate', severity: 'high', detail: `Completion rate=${completionRate.toFixed(2)} < 0.40` });
    } else if (completionRate !== null && completionRate < 0.6) {
      anomalies.push({ metric: 'agent.task.completion_rate', severity: 'medium', detail: `Completion rate=${completionRate.toFixed(2)} < 0.60` });
    }

    return {
      anomalies,
      opportunities: anomalies.length
        ? opportunities(this.domain, ['Route simpler tasks to cheaper model', 'Increase context reuse', 'Cache prompt fragments'], '-10% to -35% cost')
        : [],
    };
  }
}
