import { spawnSync } from 'child_process';
import { existsSync, readFileSync } from 'fs';
import { homedir, tmpdir } from 'os';
import { join } from 'path';
import type { PlatformAdapter } from '../core/types/adapter.js';
import type { Metric, Proposal, ExperimentResult } from '../core/types/index.js';

const CLAUDE_DATA = join(homedir(), '.claude', 'data');
const CONVERGIO_REPO = 'Roberdan/ConvergioPlatform';

/** Runs a read-only SQLite query — safe for metrics collection. */
function sqlite3(db: string, query: string): { ok: boolean; stdout: string } {
  const r = spawnSync('sqlite3', [db, query], { encoding: 'utf8' });
  return { ok: r.status === 0, stdout: r.stdout ?? '' };
}

/**
 * Adapter for .claude configuration optimisation.
 *
 * Targets: agent instruction files, skill hit-rate, token efficiency ratios.
 * Metrics sourced from ~/.claude/data/dashboard.db and session-learnings.jsonl.
 */
export class ClaudeConfigAdapter implements PlatformAdapter {
  readonly name = 'claude-config';

  /** Queries dashboard.db for session count, task completion rate, and learnings. */
  async collectMetrics(): Promise<Metric[]> {
    const now = Date.now();
    const metrics: Metric[] = [];
    const db = join(CLAUDE_DATA, 'dashboard.db');

    if (existsSync(db)) {
      const sessRes = sqlite3(db, 'SELECT COUNT(*) FROM sessions;');
      if (sessRes.ok) {
        metrics.push({
          name: 'agent.session_count',
          value: parseInt(sessRes.stdout.trim(), 10) || 0,
          timestamp: now,
          labels: { source: 'dashboard.db' },
          family: 'Agent',
        });
      }

      // Completion rate over non-pending tasks
      const doneQuery =
        "SELECT CAST(SUM(CASE WHEN status='done' THEN 1 ELSE 0 END) AS FLOAT)/COUNT(*) " +
        "FROM tasks WHERE status != 'pending';";
      const taskRes = sqlite3(db, doneQuery);
      if (taskRes.ok) {
        metrics.push({
          name: 'agent.task_completion_rate',
          value: parseFloat(taskRes.stdout.trim()) || 0,
          timestamp: now,
          labels: { source: 'plan-db' },
          family: 'Agent',
        });
      }
    }

    // Session learnings JSONL — line count proxies knowledge-capture activity
    const learningsFile = join(homedir(), '.claude', 'session-learnings.jsonl');
    if (existsSync(learningsFile)) {
      const lines = readFileSync(learningsFile, 'utf8').split('\n').filter(Boolean).length;
      metrics.push({
        name: 'agent.learnings_count',
        value: lines,
        timestamp: now,
        labels: { source: 'session-learnings.jsonl' },
        family: 'Agent',
      });
    }

    return metrics;
  }

  /**
   * Copies ~/.claude to a temp overlay, verifies dashboard.db is accessible
   * in isolation, then cleans up. Real change application is out-of-scope
   * until the proposal schema carries an explicit patch field.
   */
  async runCanary(proposal: Proposal): Promise<ExperimentResult> {
    const overlay = join(tmpdir(), `evo-claude-${proposal.id}`);
    const cpRes = spawnSync('cp', ['-r', join(homedir(), '.claude'), overlay], {
      encoding: 'utf8',
    });
    if (cpRes.status !== 0) {
      return { confidence: 0, pValue: 1, recommendation: 'Inconclusive', delta: 0, sideEffects: [] };
    }

    const dbCheck = sqlite3(join(overlay, 'data', 'dashboard.db'), 'SELECT 1;');
    spawnSync('rm', ['-rf', overlay], { encoding: 'utf8' });

    const passed = dbCheck.ok;
    return {
      confidence: passed ? 0.6 : 0.1,
      pValue: passed ? 0.1 : 0.9,
      recommendation: passed ? 'ExtendCanary' : 'Inconclusive',
      delta: 0,
      sideEffects: [],
    };
  }

  /** Opens a PR on ConvergioPlatform with claude-config/ changes. */
  async openPR(proposal: Proposal): Promise<{ prUrl: string; prNumber: number }> {
    const branch = `evo/claude/${proposal.id}`;
    const title = proposal.title || proposal.hypothesis || `Evolution proposal ${proposal.id}`;
    const target = proposal.targetMetric || `${proposal.targetAdapter}.score`;
    spawnSync('git', ['push', 'origin', branch], { encoding: 'utf8' });

    const res = spawnSync(
        'gh',
        [
          'pr', 'create', '--repo', CONVERGIO_REPO,
          '--head', branch,
          '--title', title,
          '--body', `Evolution Engine — claude-config proposal ${proposal.id}\nTarget: ${target}`,
        ],
      { encoding: 'utf8' },
    );
    if (res.status !== 0) throw new Error(`gh pr create failed: ${res.stderr}`);

    const prUrl = res.stdout.trim();
    return { prUrl, prNumber: parseInt(prUrl.split('/').at(-1) ?? '0', 10) };
  }

  /** Removes the canary temp overlay created by runCanary. */
  async rollback(experimentId: string): Promise<void> {
    const overlay = join(tmpdir(), `evo-claude-${experimentId}`);
    if (existsSync(overlay)) {
      spawnSync('rm', ['-rf', overlay], { encoding: 'utf8' });
    }
  }

  /** Confirms dashboard.db exists and responds to a ping query. */
  async healthCheck(): Promise<{ healthy: boolean; details: string }> {
    const db = join(CLAUDE_DATA, 'dashboard.db');
    if (!existsSync(db)) {
      return { healthy: false, details: `dashboard.db not found at ${db}` };
    }
    const ping = sqlite3(db, 'SELECT 1;');
    return {
      healthy: ping.ok,
      details: ping.ok ? 'dashboard.db accessible' : `sqlite3 error: ${ping.stdout.trim()}`,
    };
  }
}
