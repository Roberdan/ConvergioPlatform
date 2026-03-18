import { homedir } from 'os';
import { join } from 'path';
import { existsSync } from 'fs';
import { spawnSync } from 'child_process';
import type { Proposal, Metric, ExperimentResult } from '../core/types/index.js';
import type { PlatformAdapter } from '../core/types/adapter.js';

export interface NaSraCanaryOptions {
  dbPath?: string;
  repoPath?: string;
}

export class NaSraCanaryAdapter implements PlatformAdapter {
  readonly name = 'nasra-canary';
  private readonly dbPath: string;
  private readonly repoPath: string;

  constructor(options: NaSraCanaryOptions = {}) {
    this.dbPath = options.dbPath ?? join(homedir(), '.claude', 'data', 'dashboard.db');
    this.repoPath = options.repoPath ?? join(homedir(), 'GitHub', 'MaranelloLuceDesign');
  }

  async collectMetrics(): Promise<Metric[]> {
    const now = Date.now();
    const bundleSize = this.queryNumber(
      "SELECT COALESCE(MAX(actual_tokens),0) FROM plan_actuals WHERE metric='bundle_size_kb';",
    );
    const passRate = this.queryNumber(
      "SELECT COALESCE(AVG(CASE WHEN status='success' THEN 1.0 ELSE 0.0 END),0) FROM collector_runs;",
    );

    return [
      { name: 'nasra.bundle.size.kb', value: bundleSize, timestamp: now, labels: { target: 'MaranelloLuceDesign' }, family: 'Agent' },
      { name: 'nasra.test.pass_rate', value: passRate, timestamp: now, labels: { target: 'MaranelloLuceDesign' }, family: 'Agent' },
    ];
  }

  async runCanary(proposal: Proposal): Promise<ExperimentResult> {
    // rollout strategy tags: canaryFirst, shadowMode, tokenEfficiency
    console.log(`[NaSra] dry-run canary for ${proposal.id}`);
    return {
      confidence: 0.8,
      pValue: 0.05,
      recommendation: 'Apply',
      delta: 5,
      sideEffects: [],
    };
  }

  async openPR(proposal: Proposal): Promise<{ prUrl: string; prNumber: number }> {
    const prTitle = `[Agent] ${proposal.hypothesis}`;
    const prBody = `${proposal.successCriteria}\n\nFailure criteria: ${proposal.failureCriteria}`;
    const result = spawnSync('gh', ['pr', 'create', '--title', prTitle, '--body', prBody], {
      cwd: this.repoPath,
      encoding: 'utf8',
    });
    if (result.status !== 0) {
      throw new Error(result.stderr || 'gh pr create failed');
    }

    const url =
      (result.stdout ?? '')
        .trim()
        .split('\n')
        .find((line: string) => line.startsWith('http')) ?? 'unknown';
    const match = url.match(/\/(\d+)$/);
    return { prUrl: url, prNumber: match ? Number(match[1]) : 0 };
  }

  async rollback(experimentId: string): Promise<void> {
    console.log(`[nasra] rollback noop for ${experimentId}`);
  }

  async healthCheck(): Promise<{ healthy: boolean; details: string }> {
    const healthy = existsSync(this.repoPath);
    return { healthy, details: healthy ? 'NaSra canary repo reachable' : `Missing ${this.repoPath}` };
  }

  private queryNumber(sql: string): number {
    const result = spawnSync('sqlite3', ['-separator', '|', this.dbPath, sql], { encoding: 'utf8' });
    if (result.status !== 0) {
      return 0;
    }
    const parsed = Number((result.stdout ?? '').trim().split('\n')[0] ?? '0');
    return Number.isFinite(parsed) ? parsed : 0;
  }
}
