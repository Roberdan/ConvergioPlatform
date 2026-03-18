import type { MetricFamily } from '../core/types/index.js';

export const METRIC_FAMILIES: readonly MetricFamily[] = [
  'Runtime',
  'Mesh',
  'Database',
  'Workload',
  'Agent',
  'Build',
  'Bundle',
] as const;

export const metricFamilySignals: Record<MetricFamily, readonly string[]> = {
  Runtime: ['eventLoop', 'memory', 'cpu'],
  Mesh: ['meshTopology', 'interServiceLatency', 'peerHealth'],
  Database: ['dbQuery', 'cacheHitRate', 'lockWait'],
  Workload: ['workload', 'queueDepth', 'requestRate'],
  Agent: ['tokenUsage', 'session', 'taskFlow'],
  Build: ['buildTime', 'buildFailureRate', 'ciDuration'],
  Bundle: ['bundleSize', 'assetCount', 'chunkSkew'],
};
