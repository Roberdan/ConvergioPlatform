import type { Metric, MetricFamily } from '../core/types/index.js';
import { BUCKET_MS } from './config.js';

export interface AggregatedPoint {
  name: string;
  family: MetricFamily;
  ts: number;
  avg: number;
  min: number;
  max: number;
  count: number;
}

export function rollup5m(metrics: Metric[]): AggregatedPoint[] {
  const buckets = new Map<string, { name: string; family: MetricFamily; ts: number; values: number[] }>();

  for (const metric of metrics) {
    const ts = Math.floor(metric.timestamp / BUCKET_MS) * BUCKET_MS;
    const key = `${metric.name}:${metric.family}:${ts}`;
    const found = buckets.get(key);
    if (found) {
      found.values.push(metric.value);
      continue;
    }
    buckets.set(key, {
      name: metric.name,
      family: metric.family,
      ts,
      values: [metric.value],
    });
  }

  return [...buckets.values()]
    .map((bucket) => {
      const sum = bucket.values.reduce((total, value) => total + value, 0);
      return {
        name: bucket.name,
        family: bucket.family,
        ts: bucket.ts,
        avg: sum / bucket.values.length,
        min: Math.min(...bucket.values),
        max: Math.max(...bucket.values),
        count: bucket.values.length,
      };
    })
    .sort((left, right) => left.ts - right.ts);
}
