import type { PlatformAdapter } from '../core/types/adapter.js';
import type { Metric, Proposal, ExperimentResult } from '../core/types/index.js';

/**
 * Template adapter — copy this file to add a new PlatformAdapter.
 *
 * Steps:
 *  1. Rename `TemplateAdapter` to `<YourTarget>Adapter`
 *  2. Set `name` to a stable lowercase kebab-case identifier
 *  3. Implement each method — see inline comments for guidance
 *  4. Export from `evolution/adapters/index.ts`
 */
export class TemplateAdapter implements PlatformAdapter {
  /** Unique ID used in logs and audit trails — use lowercase kebab-case. */
  readonly name = 'template';

  constructor(
    /** Primary connection target: path, URL, or identifier */
    private readonly target: string,
  ) {}

  /**
   * Collect telemetry signals from your platform target.
   * Return one Metric per signal — values must be numeric.
   * Common families: 'Runtime', 'Build', 'Bundle', 'Workload', 'Agent'.
   */
  async collectMetrics(): Promise<Metric[]> {
    // Replace with real collection logic (read files, call APIs, shell out)
    return [
      {
        name: 'template.placeholder_metric',
        value: 0,
        timestamp: Date.now(),
        labels: { target: this.target },
        family: 'Runtime',
      },
    ];
  }

  /**
   * Apply the proposed change in a safe canary context.
   * Must honour proposal.failureCriteria and auto-rollback on breach.
   * Measure before/after delta on proposal.targetMetric.
   */
  async runCanary(proposal: Proposal): Promise<ExperimentResult> {
    // TODO: implement canary logic for this target
    void proposal;
    return {
      confidence: 0,
      pValue: 1,
      recommendation: 'Inconclusive',
      delta: 0,
      sideEffects: [],
    };
  }

  /**
   * Create a pull request applying the proposed change.
   * Pattern: `git push origin <branch>` then `gh pr create --repo <owner/repo>`.
   */
  async openPR(proposal: Proposal): Promise<{ prUrl: string; prNumber: number }> {
    // TODO: push branch and call `gh pr create`
    throw new Error(`openPR not implemented for proposal ${proposal.id}`);
  }

  /** Roll back any changes made during a canary experiment. */
  async rollback(experimentId: string): Promise<void> {
    // TODO: close PR, delete branch, restore files
    void experimentId;
  }

  /**
   * Verify the target is reachable before any operation.
   * Return `healthy: false` with a clear message when the target is unavailable.
   */
  async healthCheck(): Promise<{ healthy: boolean; details: string }> {
    // TODO: replace with a real connectivity check
    return { healthy: false, details: `${this.target} not yet verified` };
  }
}
