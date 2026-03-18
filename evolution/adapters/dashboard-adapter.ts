import { spawnSync } from 'child_process';
import { existsSync, readFileSync } from 'fs';
import { join } from 'path';
import type { PlatformAdapter } from '../core/types/adapter.js';
import type { Metric, Proposal, ExperimentResult } from '../core/types/index.js';

const CONVERGIO_REPO = 'Roberdan/ConvergioPlatform';

/**
 * Adapter for the Convergio dashboard embedding layer.
 *
 * Targets: HTTP health latency, error rate, server availability.
 * Canary strategy: before/after latency delta via `curl` probes.
 */
export class DashboardAdapter implements PlatformAdapter {
  readonly name = 'dashboard';

  constructor(
    /** Base URL of a running dashboard server, e.g. http://localhost:3000 */
    private readonly baseUrl: string,
    /** Optional local source path used for PR and rollback operations */
    private readonly sourcePath?: string,
  ) {}

  /**
   * Probes `baseUrl/__health` with curl and parses HTTP status + response time.
   * Falls back to access log parsing when sourcePath is provided.
   */
  async collectMetrics(): Promise<Metric[]> {
    const now = Date.now();
    const metrics: Metric[] = [];

    // curl -w outputs "<status_code> <time_total>" — no body needed
    const curlRes = spawnSync(
      'curl',
      ['-s', '-o', '/dev/null', '-w', '%{http_code} %{time_total}', `${this.baseUrl}/__health`],
      { encoding: 'utf8', timeout: 5000 },
    );
    if (curlRes.status === 0) {
      const [code, timeStr] = curlRes.stdout.trim().split(' ');
      const latencyMs = Math.round(parseFloat(timeStr ?? '0') * 1000);
      metrics.push({
        name: 'web.health_latency_ms',
        value: latencyMs,
        timestamp: now,
        labels: { url: this.baseUrl, status: code ?? '0' },
        family: 'Runtime',
      });
      metrics.push({
        name: 'web.health_ok',
        value: code === '200' ? 1 : 0,
        timestamp: now,
        labels: { url: this.baseUrl },
        family: 'Runtime',
      });
    }

    // Parse 5xx error rate from access log when source is available
    if (this.sourcePath) {
      const logFile = join(this.sourcePath, 'logs', 'access.log');
      if (existsSync(logFile)) {
        const lines = readFileSync(logFile, 'utf8').split('\n').filter(Boolean);
        const errors = lines.filter((l: string) => / 5\d\d /.test(l)).length;
        metrics.push({
          name: 'web.error_rate',
          value: lines.length > 0 ? errors / lines.length : 0,
          timestamp: now,
          labels: { url: this.baseUrl },
          family: 'Runtime',
        });
      }
    }

    return metrics;
  }

  /**
   * Takes a baseline probe, waits 2 s for any in-flight changes to settle,
   * re-probes, and returns latency delta as the canary signal.
   */
  async runCanary(proposal: Proposal): Promise<ExperimentResult> {
    const before = await this.collectMetrics();
    const latencyBefore = before.find((m) => m.name === 'web.health_latency_ms')?.value ?? 0;

    await new Promise<void>((resolve) => setTimeout(resolve, 2000));

    const after = await this.collectMetrics();
    const latencyAfter = after.find((m) => m.name === 'web.health_latency_ms')?.value ?? 0;

    const delta = latencyBefore > 0 ? (latencyAfter - latencyBefore) / latencyBefore : 0;
    const maxDelta = proposal.expectedDelta?.max ?? -0.05;
    const improved = delta < maxDelta;

    return {
      confidence: improved ? 0.7 : 0.2,
      pValue: improved ? 0.05 : 0.8,
      recommendation: improved ? 'Apply' : 'Reject',
      delta,
      sideEffects: [],
    };
  }

  /** Pushes the experiment branch and opens a PR on ConvergioPlatform. */
  async openPR(proposal: Proposal): Promise<{ prUrl: string; prNumber: number }> {
    const branch = `evo/dashboard/${proposal.id}`;
    const title = proposal.title || proposal.hypothesis || `Evolution proposal ${proposal.id}`;
    const target = proposal.targetMetric || `${proposal.targetAdapter}.score`;
    if (this.sourcePath) {
      spawnSync('git', ['push', 'origin', branch], { cwd: this.sourcePath, encoding: 'utf8' });
    }
    const res = spawnSync(
        'gh',
        [
          'pr', 'create', '--repo', CONVERGIO_REPO,
          '--head', branch,
          '--title', title,
          '--body', `Evolution Engine — dashboard proposal ${proposal.id}\nTarget: ${target}`,
        ],
      { encoding: 'utf8' },
    );
    if (res.status !== 0) throw new Error(`gh pr create failed: ${res.stderr}`);

    const prUrl = res.stdout.trim();
    return { prUrl, prNumber: parseInt(prUrl.split('/').at(-1) ?? '0', 10) };
  }

  /** Checks out main and deletes the experiment branch from the local clone. */
  async rollback(experimentId: string): Promise<void> {
    if (!this.sourcePath) return;
    const branch = `evo/dashboard/${experimentId}`;
    spawnSync('git', ['checkout', 'main'], { cwd: this.sourcePath, encoding: 'utf8' });
    spawnSync('git', ['branch', '-D', branch], { cwd: this.sourcePath, encoding: 'utf8' });
  }

  /** GETs `baseUrl/__health` — healthy when HTTP 200. */
  async healthCheck(): Promise<{ healthy: boolean; details: string }> {
    const res = spawnSync(
      'curl',
      ['-s', '-o', '/dev/null', '-w', '%{http_code}', `${this.baseUrl}/__health`],
      { encoding: 'utf8', timeout: 5000 },
    );
    if (res.status !== 0) {
      return { healthy: false, details: `curl failed: ${res.stderr?.trim()}` };
    }
    const code = res.stdout.trim();
    return { healthy: code === '200', details: `HTTP ${code} from ${this.baseUrl}/__health` };
  }
}
