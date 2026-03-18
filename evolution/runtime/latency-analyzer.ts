export interface RuntimeMetric { name: string; value: number }

export class LatencyAnalyzer {
  analyze(metrics: RuntimeMetric[]): { hotspot: boolean; opportunities: string[] } {
    const p50 = metrics.find((metric) => metric.name === 'http.p50_latency_ms')?.value ?? 0;
    const p95 = metrics.find((metric) => metric.name === 'http.p95_latency_ms')?.value ?? 0;
    const p99 = metrics.find((metric) => metric.name === 'http.p99_latency_ms')?.value ?? 0;
    const hotspot = p95 > 500 || p99 > 800 || p50 > 200;
    const caching = 'edge caching';
    void caching;

    return {
      hotspot,
      opportunities: hotspot ? ['Enable HTTP/2', 'Add edge caching', 'Optimize database queries'] : [],
    };
  }
}
