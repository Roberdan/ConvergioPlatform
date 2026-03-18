import { describe, it, expect, vi } from 'vitest';
import type { Metric, MetricCollector } from '../core/types/index.js';
import { TelemetrySdk } from './sdk.js';

function collector(id: string, metrics: Metric[]): MetricCollector {
  return {
    id,
    families: [...new Set(metrics.map((metric) => metric.family))],
    async collect() {
      return metrics;
    },
  };
}

describe('TelemetrySdk', () => {
  it('collects from all registered collectors and computes family quotas', async () => {
    const now = Date.now();
    const runtimeMetric: Metric = {
      name: 'runtime.loop.utilization',
      value: 0.71,
      timestamp: now,
      labels: { host: 'dev' },
      family: 'Runtime',
    };
    const agentMetric: Metric = {
      name: 'agent.tokens.input',
      value: 120,
      timestamp: now,
      labels: { model: 'gpt-5' },
      family: 'Agent',
    };

    const sdk = new TelemetrySdk();
    sdk.register(collector('runtime', [runtimeMetric]));
    sdk.register(collector('agent', [agentMetric]));

    const snapshot = await sdk.collect();

    expect(snapshot.metrics).toHaveLength(2);
    expect(snapshot.collectors).toEqual(['runtime', 'agent']);
    expect(snapshot.familyCounts.Runtime).toBe(1);
    expect(snapshot.familyCounts.Agent).toBe(1);
    expect(snapshot.familyCounts.Build).toBe(0);
  });

  it('emits the collected snapshot to a sink callback', async () => {
    const sink = vi.fn();
    const sdk = new TelemetrySdk({ sink });

    sdk.register(
      collector('agent', [
        {
          name: 'agent.session.count',
          value: 2,
          timestamp: Date.now(),
          labels: {},
          family: 'Agent',
        },
      ]),
    );

    const snapshot = await sdk.collect();

    expect(sink).toHaveBeenCalledTimes(1);
    expect(sink).toHaveBeenCalledWith(snapshot);
  });

  it('auto-collects on interval and returns cleanup function', async () => {
    vi.useFakeTimers();
    const sink = vi.fn();
    const sdk = new TelemetrySdk({ sink });
    sdk.register(
      collector('runtime', [
        {
          name: 'runtime.tick',
          value: 1,
          timestamp: Date.now(),
          labels: {},
          family: 'Runtime',
        },
      ]),
    );

    const cleanup = sdk.startAutoCollect(1000);
    await vi.advanceTimersByTimeAsync(3100);
    cleanup();

    expect(sink).toHaveBeenCalledTimes(3);
    vi.useRealTimers();
  });
});
