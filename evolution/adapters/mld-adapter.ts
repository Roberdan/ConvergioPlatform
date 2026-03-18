import { spawnSync } from 'child_process';
import { existsSync, readdirSync, readFileSync, statSync } from 'fs';
import { join } from 'path';
import type { PlatformAdapter } from '../core/types/adapter.js';
import type { Metric, Proposal, ExperimentResult } from '../core/types/index.js';

const MLD_REPO = 'Roberdan/MaranelloLuceDesign';

/** Thin `gh` CLI wrapper — returns structured result instead of throwing. */
function gh(...args: string[]): { ok: boolean; stdout: string; stderr: string } {
  const r = spawnSync('gh', args, { encoding: 'utf8' });
  return { ok: r.status === 0, stdout: r.stdout ?? '', stderr: r.stderr ?? '' };
}

/**
 * Adapter for MaranelloLuceDesign — Ferrari Luce design system.
 *
 * Targets: JS bundle size, CI build duration, test pass rate.
 * Canary strategy: feature branch → draft PR → poll CI checks → verdict.
 */
export class MLDAdapter implements PlatformAdapter {
  readonly name = 'mld';

  constructor(
    /** Absolute path to a local clone of MaranelloLuceDesign */
    private readonly repoPath: string,
  ) {}

  /** Reads dist/ JS sizes and last CI run duration via `gh run list`. */
  async collectMetrics(): Promise<Metric[]> {
    const now = Date.now();
    const metrics: Metric[] = [];

    const distDir = join(this.repoPath, 'dist');
    if (existsSync(distDir)) {
      const totalBytes = readdirSync(distDir)
        .filter((f: string) => f.endsWith('.js'))
        .reduce((sum: number, f: string) => sum + statSync(join(distDir, f)).size, 0);
      metrics.push({
        name: 'bundle.total_js_kb',
        value: Math.round(totalBytes / 1024),
        timestamp: now,
        labels: { repo: MLD_REPO },
        family: 'Bundle',
      });
    }

    // Last completed CI run — gives build duration and conclusion
    const runRes = gh('run', 'list', '--repo', MLD_REPO, '--limit', '1', '--json', 'durationMs,conclusion');
    if (runRes.ok) {
      type Run = { durationMs: number; conclusion: string };
      const [run] = JSON.parse(runRes.stdout) as Run[];
      if (run) {
        metrics.push({
          name: 'ci.duration_ms',
          value: run.durationMs,
          timestamp: now,
          labels: { repo: MLD_REPO, conclusion: run.conclusion },
          family: 'Build',
        });
      }
    }

    // Vitest JSON report when a local build exists
    const reportFile = join(this.repoPath, 'test-results', 'results.json');
    if (existsSync(reportFile)) {
      type Report = { numTotalTests: number; numPassedTests: number };
      const report = JSON.parse(readFileSync(reportFile, 'utf8')) as Report;
      metrics.push({
        name: 'tests.pass_rate',
        value: report.numTotalTests > 0 ? report.numPassedTests / report.numTotalTests : 0,
        timestamp: now,
        labels: { repo: MLD_REPO },
        family: 'Workload',
      });
    }

    return metrics;
  }

  /**
   * Pushes `evo/canary/<id>` as a draft PR and polls CI for up to 5 min.
   * Returns confidence derived from check conclusions.
   */
  async runCanary(proposal: Proposal): Promise<ExperimentResult> {
    const branch = `evo/canary/${proposal.id}`;
    const git = (...a: string[]) =>
      spawnSync('git', a, { cwd: this.repoPath, encoding: 'utf8' });

    git('checkout', '-b', branch);
    if (git('push', 'origin', branch).status !== 0) {
      return { confidence: 0, pValue: 1, recommendation: 'Inconclusive', delta: 0, sideEffects: [] };
    }

    const prRes = gh(
      'pr', 'create', '--repo', MLD_REPO,
      '--head', branch, '--draft',
      '--title', `[canary] ${proposal.hypothesis}`,
      '--body', `Auto-created by Evolution Engine\nProposal: ${proposal.id}`,
    );
    if (!prRes.ok) {
      return { confidence: 0, pValue: 1, recommendation: 'Inconclusive', delta: 0, sideEffects: [] };
    }

    const prNumber = prRes.stdout.trim().split('/').at(-1) ?? '';
    // Poll for CI completion — max 30 × 10 s = 5 min
    for (let i = 0; i < 30; i++) {
      await new Promise<void>((resolve) => setTimeout(resolve, 10_000));
      const checks = gh('pr', 'checks', prNumber, '--repo', MLD_REPO);
      if (checks.ok && !/pending|queued/i.test(checks.stdout)) break;
    }

    const viewRes = gh('pr', 'view', prNumber, '--repo', MLD_REPO, '--json', 'statusCheckRollup');
    const passed = viewRes.ok && !/FAILURE|ERROR/i.test(viewRes.stdout);

    return {
      confidence: passed ? 0.8 : 0.1,
      pValue: passed ? 0.04 : 0.9,
      recommendation: passed ? 'Apply' : 'Reject',
      delta: 0,
      sideEffects: [],
    };
  }

  /** Pushes `evo/<id>` and opens a real PR on MLD repo. */
  async openPR(proposal: Proposal): Promise<{ prUrl: string; prNumber: number }> {
    const branch = `evo/${proposal.id}`;
    spawnSync('git', ['push', 'origin', branch], { cwd: this.repoPath, encoding: 'utf8' });

    const res = gh(
      'pr', 'create', '--repo', MLD_REPO,
      '--head', branch,
      '--title', proposal.hypothesis,
      '--body', `Evolution Engine — proposal ${proposal.id}\nTarget: ${proposal.targetMetric}`,
    );
    if (!res.ok) throw new Error(`gh pr create failed: ${res.stderr}`);

    const prUrl = res.stdout.trim();
    return { prUrl, prNumber: parseInt(prUrl.split('/').at(-1) ?? '0', 10) };
  }

  /** Closes the canary PR and deletes the remote branch. */
  async rollback(experimentId: string): Promise<void> {
    const branch = `evo/canary/${experimentId}`;
    const listRes = gh('pr', 'list', '--repo', MLD_REPO, '--head', branch, '--json', 'number');
    if (listRes.ok) {
      type PR = { number: number };
      const [pr] = JSON.parse(listRes.stdout) as PR[];
      if (pr) {
        gh('pr', 'close', String(pr.number), '--repo', MLD_REPO,
          '--comment', 'Auto-rolled back by Evolution Engine');
      }
    }
    spawnSync('git', ['push', 'origin', '--delete', branch], {
      cwd: this.repoPath,
      encoding: 'utf8',
    });
  }

  /** Confirms repoPath is a git repo and last CI run succeeded. */
  async healthCheck(): Promise<{ healthy: boolean; details: string }> {
    if (!existsSync(join(this.repoPath, '.git'))) {
      return { healthy: false, details: `Not a git repo: ${this.repoPath}` };
    }
    const runRes = gh('run', 'list', '--repo', MLD_REPO, '--limit', '1', '--json', 'conclusion');
    if (!runRes.ok) return { healthy: false, details: `gh error: ${runRes.stderr.trim()}` };

    type Run = { conclusion: string };
    const [run] = JSON.parse(runRes.stdout) as Run[];
    return {
      healthy: run?.conclusion === 'success',
      details: `Last CI: ${run?.conclusion ?? 'unknown'}`,
    };
  }
}
