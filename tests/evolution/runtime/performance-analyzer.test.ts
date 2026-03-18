import { describe, expect, it } from 'vitest';
import { PerformanceAnalyzer } from '../../../src/ts/evolution/runtime/performance-analyzer.ts';

describe('PerformanceAnalyzer', () => {
  it('flags bundle, build, startup, and memory regressions', () => {
    const analyzer = new PerformanceAnalyzer();
    const result = analyzer.analyze([
      { name: 'bundle.js_size_bytes', value: 650 * 1024, timestamp: Date.now(), labels: {}, family: 'Bundle' },
      { name: 'build.duration_ms', value: 150_000, timestamp: Date.now(), labels: {}, family: 'Build' },
      { name: 'runtime.startup_ms', value: 2500, timestamp: Date.now(), labels: {}, family: 'Runtime' },
      { name: 'runtime.memory_mb', value: 9_000, timestamp: Date.now(), labels: {}, family: 'Runtime' },
    ]);

    expect(result.anomalies).toContain('bundleSize');
    expect(result.anomalies).toContain('buildTime');
    expect(result.anomalies).toContain('startup');
    expect(result.anomalies).toContain('memoryFootprint');
  });
});
