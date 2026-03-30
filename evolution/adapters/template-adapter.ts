import { spawnSync } from 'child_process';
import type { PlatformAdapter } from '../core/types/adapter.js';
import type { Metric, Proposal, ExperimentResult } from '../core/types/index.js';

const DAEMON_HEALTH_URL = 'http://localhost:8420/api/health';

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
    /** Primary connection target: path to a local git repo clone */
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
   * Override in subclasses with target-specific canary logic.
   */
  async runCanary(proposal: Proposal): Promise<ExperimentResult> {
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
   * Pushes the current branch to origin and opens a PR via `gh pr create`.
   * Reads repo and head branch from the local git context in `this.target`.
   */
  async openPR(proposal: Proposal): Promise<{ prUrl: string; prNumber: number }> {
    const branch = `evo/template/${proposal.id}`;
    const title = proposal.title || proposal.hypothesis || `Evolution proposal ${proposal.id}`;
    const target = proposal.targetMetric || `${proposal.targetAdapter}.score`;

    spawnSync('git', ['push', 'origin', branch], { cwd: this.target, encoding: 'utf8' });

    const res = spawnSync(
      'gh',
      [
        'pr', 'create',
        '--head', branch,
        '--title', title,
        '--body', `Evolution Engine — template proposal ${proposal.id}\nTarget: ${target}`,
      ],
      { cwd: this.target, encoding: 'utf8' },
    );
    if (res.status !== 0) throw new Error(`gh pr create failed: ${res.stderr?.trim()}`);

    const prUrl = res.stdout.trim();
    return { prUrl, prNumber: parseInt(prUrl.split('/').at(-1) ?? '0', 10) };
  }

  /**
   * Reverts the last commit on the current branch via `git revert HEAD`.
   * Non-interactive: uses --no-edit to avoid prompting.
   */
  async rollback(experimentId: string): Promise<void> {
    const res = spawnSync(
      'git', ['revert', 'HEAD', '--no-edit'],
      { cwd: this.target, encoding: 'utf8' },
    );
    if (res.status !== 0) {
      throw new Error(`git revert failed for experiment ${experimentId}: ${res.stderr?.trim()}`);
    }
  }

  /**
   * Probes the Convergio daemon health endpoint.
   * Returns healthy=true only when the daemon responds with HTTP 200.
   */
  async healthCheck(): Promise<{ healthy: boolean; details: string }> {
    const res = spawnSync(
      'curl',
      ['-s', '-o', '/dev/null', '-w', '%{http_code}', '--max-time', '5', DAEMON_HEALTH_URL],
      { encoding: 'utf8' },
    );
    if (res.status !== 0) {
      return { healthy: false, details: `curl failed: ${res.stderr?.trim()}` };
    }
    const code = res.stdout.trim();
    return { healthy: code === '200', details: `HTTP ${code} from ${DAEMON_HEALTH_URL}` };
  }
}
