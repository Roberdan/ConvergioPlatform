import type { Metric, MetricFamily, MetricCollector } from '../core/types/index.js';

export type TelemetrySinkFn = (snapshot: TelemetrySnapshot) => void | Promise<void>;

export interface CollectorRegistration {
  collector: MetricCollector;
  registeredAt: number;
}

export interface FamilyQuota {
  limit: number;
  used: number;
  remaining: number;
}

export type FamilyQuotaMap = Record<MetricFamily, FamilyQuota>;
export type FamilyCountMap = Record<MetricFamily, number>;

export interface TelemetrySnapshot {
  collectedAt: number;
  metrics: Metric[];
  collectors: string[];
  collectorCount: number;
  familyCounts: FamilyCountMap;
  familyQuota: FamilyQuotaMap;
}

export interface TelemetrySdkOptions {
  sink?: TelemetrySinkFn;
  quotas?: Partial<Record<MetricFamily, number>>;
}
