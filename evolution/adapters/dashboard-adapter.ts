import type { PlatformAdapter } from '../core/types/adapter.js';
import type { Metric, Proposal, ExperimentResult } from '../core/types/index.js';

/**
 * Adapter for the Convergio dashboard UI embedding layer.
 *
 * Targets: Maranello WC render performance, dashboard bundle size,
 *          LCP / FID from synthetic Playwright runs, a11y compliance.
 * Canary strategy: swap WC version in demo/, run Lighthouse CI diff.
 */
export class DashboardAdapter implements PlatformAdapter {
  readonly name = 'dashboard';

  constructor(
    /** URL or path to a running dashboard dev server */
    private readonly baseUrl: string,
  ) {}

  /**
   * Hits `baseUrl/__health` and the Lighthouse CI endpoint to
   * produce Core Web Vitals and bundle metrics.
   */
  async collectMetrics(): Promise<Metric[]> {
    const now = Date.now();
    // Concrete: fetch LCP/FID/CLS from Lighthouse JSON output
    return [
      {
        name: 'web.lcp_ms',
        value: 0,
        timestamp: now,
        labels: { target: this.baseUrl, route: '/' },
        family: 'Runtime',
      },
      {
        name: 'web.cls_score',
        value: 0,
        timestamp: now,
        labels: { target: this.baseUrl, route: '/' },
        family: 'Runtime',
      },
      {
        name: 'web.accessibility_score',
        value: 0,
        timestamp: now,
        labels: { target: this.baseUrl, suite: 'lighthouse' },
        family: 'Workload',
      },
    ];
  }

  /**
   * Spins up a preview deployment, routes a canary traffic split,
   * and compares Core Web Vitals between control and treatment.
   */
  async runCanary(proposal: Proposal): Promise<ExperimentResult> {
    return {
      confidence: 0,
      pValue: 1,
      recommendation: 'Inconclusive',
      delta: 0,
      sideEffects: [],
    };
  }

  /**
   * Opens a PR in the dashboard repo with updated WC references
   * or CSS token overrides from the proposal.
   */
  async openPR(proposal: Proposal): Promise<{ prUrl: string; prNumber: number }> {
    throw new Error(`openPR not yet implemented for proposal ${proposal.id}`);
  }

  /** Reverts a canary deployment by switching traffic back to control. */
  async rollback(experimentId: string): Promise<void> {
    throw new Error(`rollback not yet implemented for experiment ${experimentId}`);
  }

  /** GETs `baseUrl/__health` — expects 200 with `{ ok: true }`. */
  async healthCheck(): Promise<{ healthy: boolean; details: string }> {
    return { healthy: false, details: `${this.baseUrl}/__health not yet reachable` };
  }
}
