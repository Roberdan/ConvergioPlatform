import type { AggregatedPoint } from '../../../telemetry/aggregation.js';
import type { Metric, EvaluationResult } from '../../types/index.js';
import { BaseEvaluator } from '../base-evaluator.js';
import { latestValue, opportunities } from './shared.js';

const MB = 1024 * 1024;

export class DbEvaluator extends BaseEvaluator {
  readonly domain = 'database';
  readonly metricFamilies = ['Database'] as const;

  protected async analyze(metrics: Metric[], _history: AggregatedPoint[]): Promise<Partial<EvaluationResult>> {
    const anomalies: EvaluationResult['anomalies'] = [];
    const queryP95 = latestValue(metrics, 'db.query_p95_ms');
    const poolUsage = latestValue(metrics, 'db.connection_pool_usage');
    const walSize = latestValue(metrics, 'db.wal_size_bytes');

    if (queryP95 !== null && queryP95 > 500) {
      anomalies.push({ metric: 'db.query_p95_ms', severity: 'high', detail: `Query p95=${queryP95}ms > 500ms` });
    } else if (queryP95 !== null && queryP95 > 200) {
      anomalies.push({ metric: 'db.query_p95_ms', severity: 'medium', detail: `Query p95=${queryP95}ms > 200ms` });
    }

    const poolPct = poolUsage !== null && poolUsage <= 1 ? poolUsage * 100 : poolUsage;
    if (poolPct !== null && poolPct > 80) {
      anomalies.push({ metric: 'db.connection_pool_usage', severity: 'medium', detail: `Pool usage=${poolPct.toFixed(1)}% > 80%` });
    }

    if (walSize !== null && walSize > 100 * MB) {
      anomalies.push({ metric: 'db.wal_size_bytes', severity: 'medium', detail: `WAL size=${(walSize / MB).toFixed(1)}MB > 100MB` });
    }

    return {
      anomalies,
      opportunities: anomalies.length
        ? opportunities(this.domain, ['Add missing index', 'Increase connection pool', 'Schedule WAL checkpoint'], '-10% to -35% DB contention')
        : [],
    };
  }
}
