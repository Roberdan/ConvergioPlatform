import type { PlatformAdapter } from '../core/types/adapter.js';
import type { Metric, Proposal, ExperimentResult } from '../core/types/index.js';

/**
 * Adapter for .claude config optimisation.
 *
 * Targets: agent instruction files, skill definitions, MCP server configs.
 * Metrics: token efficiency ratios, skill hit-rate, agent task-completion rate.
 */
export class ClaudeConfigAdapter implements PlatformAdapter {
  readonly name = 'claude-config';

  /**
   * Reads token-usage logs from ~/.claude/logs/ and session store
   * to derive agent efficiency metrics.
   */
  async collectMetrics(): Promise<Metric[]> {
    const now = Date.now();
    // Concrete implementation will parse ~/.claude/data/dashboard.db
    // and aggregate per-skill token ratios, task success rates.
    return [
      {
        name: 'agent.skill_hit_rate',
        value: 0,
        timestamp: now,
        labels: { provider: 'claude', target: 'skills' },
        family: 'Agent',
      },
      {
        name: 'agent.token_efficiency',
        value: 0,
        timestamp: now,
        labels: { provider: 'claude', target: 'instructions' },
        family: 'Agent',
      },
    ];
  }

  /**
   * Applies a proposed instruction/skill change in a feature branch,
   * runs a shadow session, and compares task-completion metrics.
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
   * Opens a PR in the .claude config repository with the proposed change.
   * Uses `gh pr create` via child_process under the hood.
   */
  async openPR(proposal: Proposal): Promise<{ prUrl: string; prNumber: number }> {
    throw new Error(`openPR not yet implemented for proposal ${proposal.id}`);
  }

  /** Confirms ~/.claude directory is readable and dashboard.db exists. */
  async healthCheck(): Promise<{ healthy: boolean; details: string }> {
    return { healthy: false, details: 'Not yet wired to ~/.claude/data/dashboard.db' };
  }
}
