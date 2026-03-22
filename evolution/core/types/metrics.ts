/**
 * Metric types for the Evolution Engine.
 * Imported by collectors and adapters that emit telemetry.
 */

// ── Metric ────────────────────────────────────────────────────────────────────

/** Telemetry signal categories collected across platform targets. */
export type MetricFamily =
  | 'Runtime'
  | 'Mesh'
  | 'Database'
  | 'Workload'
  | 'Agent'
  | 'Build'
  | 'Bundle';

/**
 * A single telemetry reading with optional dimension labels.
 * Values are always numeric; semantics are encoded in `name` and `family`.
 */
export interface Metric {
  /** Dot-namespaced name, e.g. `http.p95_latency_ms` */
  name: string;
  /** Raw numeric value at collection time */
  value: number;
  /** Unix epoch milliseconds */
  timestamp: number;
  /** Arbitrary key-value dimensions (service, region, env, …) */
  labels: Record<string, string>;
  /** High-level grouping for routing and budget tracking */
  family: MetricFamily;
}

/**
 * Pluggable metric source that the engine polls on each evaluation cycle.
 * Adapters implement this to feed domain-specific telemetry into the core.
 */
export interface MetricCollector {
  /** Stable collector identifier, e.g. `lighthouse`, `vitest-coverage` */
  readonly id: string;
  /** Metric families this collector can produce */
  readonly families: readonly MetricFamily[];
  /** Gather metrics from the underlying source. */
  collect(): Promise<Metric[]>;
}
