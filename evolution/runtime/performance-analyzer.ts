export interface RuntimeMetric { name: string; value: number }

export class PerformanceAnalyzer {
  analyze(metrics: RuntimeMetric[]): { anomalies: string[]; opportunities: string[] } {
    const bundleSize = metrics.find((metric) => metric.name === 'bundle.js_size_bytes')?.value ?? 0;
    const buildTime = metrics.find((metric) => metric.name === 'build.duration_ms')?.value ?? 0;
    const startup = metrics.find((metric) => metric.name === 'runtime.startup_ms')?.value ?? 0;
    const memoryFootprint = metrics.find((metric) => metric.name === 'runtime.memory_mb')?.value ?? 0;

    const anomalies: string[] = [];
    if (bundleSize > 500 * 1024) anomalies.push('bundleSize');
    if (buildTime > 120_000) anomalies.push('buildTime');
    if (startup > 2_000) anomalies.push('startup');
    if (memoryFootprint > 8_000) anomalies.push('memoryFootprint');

    return {
      anomalies,
      opportunities: anomalies.length ? ['Tree-shake unused imports', 'Enable code splitting', 'Lazy-load routes'] : [],
    };
  }
}
