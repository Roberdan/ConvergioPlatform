export interface RuntimeMetric { name: string; value: number }

export class WorkloadAnalyzer {
  analyze(metrics: RuntimeMetric[]): { anomalies: string[]; suggestions: string[] } {
    const loadImbalance = metrics.find((metric) => metric.name === 'workload.load_imbalance')?.value ?? 0;
    const queueDepth = metrics.find((metric) => metric.name === 'workload.queue_depth')?.value ?? 0;

    const anomalies: string[] = [];
    if (loadImbalance > 0.3) anomalies.push('loadImbalance');
    if (queueDepth > 100) anomalies.push('queueDepth');

    return {
      anomalies,
      suggestions: anomalies.length ? ['Horizontal scaling', 'Workload rebalancing'] : ['No scaling required'],
    };
  }
}
