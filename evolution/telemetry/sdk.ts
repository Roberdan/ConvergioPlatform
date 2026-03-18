import type { Metric, MetricCollector, MetricFamily } from '../core/types/index.js';
import { CollectorRegistry } from './collectors.js';
import { METRIC_FAMILIES } from './metric-families.js';
import type {
  FamilyCountMap,
  FamilyQuotaMap,
  TelemetrySdkOptions,
  TelemetrySnapshot,
} from './types.js';

const DEFAULT_QUOTA = 1000;

export class TelemetrySdk {
  private readonly registry = new CollectorRegistry();
  private readonly sink?: TelemetrySdkOptions['sink'];
  private readonly quotas: Record<MetricFamily, number>;
  private intervalHandle?: ReturnType<typeof setInterval>;

  constructor(options: TelemetrySdkOptions = {}) {
    this.sink = options.sink;
    this.quotas = METRIC_FAMILIES.reduce<Record<MetricFamily, number>>((acc, family) => {
      acc[family] = options.quotas?.[family] ?? DEFAULT_QUOTA;
      return acc;
    }, {} as Record<MetricFamily, number>);
  }

  register(collector: MetricCollector): void {
    this.registry.register(collector);
  }

  unregister(collectorId: string): void {
    this.registry.unregister(collectorId);
  }

  async collect(): Promise<TelemetrySnapshot> {
    const collectedAt = Date.now();
    const metricsByCollector = await Promise.all(this.registry.list().map(async (collector) => collector.collect()));
    const metrics = metricsByCollector.flatMap((collectorMetrics) => collectorMetrics);
    const familyCounts = this.computeFamilyCounts(metrics);
    const snapshot: TelemetrySnapshot = {
      collectedAt,
      metrics,
      collectors: this.registry.ids(),
      collectorCount: this.registry.size(),
      familyCounts,
      familyQuota: this.computeQuota(familyCounts),
    };

    if (this.sink) {
      await this.sink(snapshot);
    }

    return snapshot;
  }

  startAutoCollect(intervalMs: number): () => void {
    if (intervalMs <= 0) {
      throw new Error('intervalMs must be greater than 0');
    }

    if (this.intervalHandle) {
      clearInterval(this.intervalHandle);
    }

    this.intervalHandle = setInterval(() => {
      void this.collect();
    }, intervalMs);

    return () => {
      if (this.intervalHandle) {
        clearInterval(this.intervalHandle);
        this.intervalHandle = undefined;
      }
    };
  }

  private computeFamilyCounts(metrics: Metric[]): FamilyCountMap {
    const counts = METRIC_FAMILIES.reduce<FamilyCountMap>((acc, family) => {
      acc[family] = 0;
      return acc;
    }, {} as FamilyCountMap);

    for (const metric of metrics) {
      counts[metric.family] += 1;
    }

    return counts;
  }

  private computeQuota(familyCounts: FamilyCountMap): FamilyQuotaMap {
    return METRIC_FAMILIES.reduce<FamilyQuotaMap>((acc, family) => {
      const used = familyCounts[family];
      const limit = this.quotas[family];
      acc[family] = {
        limit,
        used,
        remaining: Math.max(limit - used, 0),
      };
      return acc;
    }, {} as FamilyQuotaMap);
  }
}
