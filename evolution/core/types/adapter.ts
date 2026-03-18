import type { Metric, Proposal, ExperimentResult } from './index.js';

/**
 * Thin adapter contract — each repo/project implements this interface.
 *
 * Adapters are the only place that knows about a specific target's
 * file layout, CI system, or observability stack. Core engine code
 * must remain adapter-agnostic.
 */
export interface PlatformAdapter {
  /** Unique identifier for this adapter, e.g. `maranello`, `claude-config` */
  readonly name: string;

  /** Collect telemetry signals from this platform target. */
  collectMetrics(): Promise<Metric[]>;

  /**
   * Run a canary experiment scoped to this target.
   * Implementations must respect `proposal.failureCriteria` and auto-rollback.
   */
  runCanary(proposal: Proposal): Promise<ExperimentResult>;

  /**
   * Open a pull request applying the proposed change.
   * Returns the PR URL and number for audit logging.
   */
  openPR(proposal: Proposal): Promise<{ prUrl: string; prNumber: number }>;

  /** Verify the adapter's target is reachable and healthy before operations. */
  healthCheck(): Promise<{ healthy: boolean; details: string }>;
}
