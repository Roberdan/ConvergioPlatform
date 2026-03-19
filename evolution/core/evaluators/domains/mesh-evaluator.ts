import type { AggregatedPoint } from '../../../telemetry/aggregation.js';
import type { Metric, EvaluationResult } from '../../types/index.js';
import { BaseEvaluator } from '../base-evaluator.js';
import { latestValue, opportunities } from './shared.js';

export class MeshEvaluator extends BaseEvaluator {
  readonly domain = 'mesh';
  readonly metricFamilies = ['Mesh', 'Database'] as const;

  protected async analyze(metrics: Metric[], _history: AggregatedPoint[]): Promise<Partial<EvaluationResult>> {
    const anomalies: EvaluationResult['anomalies'] = [];
    const syncLagMs = latestValue(metrics, 'mesh.sync_lag_ms');
    const dbP95 = latestValue(metrics, 'db.query_p95_ms');

    if (syncLagMs !== null && syncLagMs > 120_000) {
      anomalies.push({ metric: 'mesh.sync_lag_ms', severity: 'high', detail: `Sync lag=${syncLagMs}ms > 120000ms` });
    } else if (syncLagMs !== null && syncLagMs > 30_000) {
      anomalies.push({ metric: 'mesh.sync_lag_ms', severity: 'medium', detail: `Sync lag=${syncLagMs}ms > 30000ms` });
    }
    if (dbP95 !== null && dbP95 > 200) {
      anomalies.push({ metric: 'db.query_p95_ms', severity: 'medium', detail: `DB p95=${dbP95}ms > 200ms` });
    }

    return {
      anomalies,
      opportunities: anomalies.length
        ? opportunities(this.domain, ['Optimize sync batch size', 'Add DB index', 'Increase connection pool'], '-10% to -30% sync lag')
        : [],
    };
  }
}
