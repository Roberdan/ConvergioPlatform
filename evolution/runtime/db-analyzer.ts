export interface RuntimeMetric { name: string; value: number }

export class DbAnalyzer {
  analyze(metrics: RuntimeMetric[]): { anomalies: string[]; opportunities: string[] } {
    const slowQuery = metrics.find((metric) => metric.name === 'db.query_p95_ms')?.value ?? 0;
    const indexUsage = metrics.find((metric) => metric.name === 'db.index_usage')?.value ?? 1;
    const connectionPool = metrics.find((metric) => metric.name === 'db.connection_pool_usage')?.value ?? 0;

    const anomalies: string[] = [];
    if (slowQuery > 200) anomalies.push('slowQuery');
    if (indexUsage < 0.7) anomalies.push('indexUsage');
    if (connectionPool > 0.8) anomalies.push('connectionPool');

    return {
      anomalies,
      opportunities: anomalies.length ? ['Add missing index', 'Increase connection pool', 'Schedule WAL checkpoint'] : [],
    };
  }
}
