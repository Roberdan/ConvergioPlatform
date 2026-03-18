/**
 * Scoreboard — aggregates proposal lifecycle statistics.
 *
 * Counts proposals by status: generated, accepted (Applied), rejected.
 * Used for governance reporting and dashboard ROI overview.
 */

import { spawnSync } from 'child_process';
import { existsSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';

/** Aggregate proposal counts per period. */
export interface ProposalStats {
  period: string;
  proposalsGenerated: number;
  proposalsAccepted: number;
  proposalsRejected: number;
  /** Ratio accepted / generated [0-1] */
  acceptanceRate: number;
}

/** Ranked proposal entry by experiment delta score. */
export interface ProposalScore {
  proposalId: string;
  title: string;
  deltaScore: number;
  mode: string;
  completedAt: number;
}

export interface ScoreboardOptions {
  dbPath?: string;
}

function sqlite3Q(dbPath: string, sql: string): string {
  const r = spawnSync('sqlite3', [dbPath, sql], { encoding: 'utf-8' });
  return r.status === 0 ? (r.stdout ?? '').trim() : '';
}

export class Scoreboard {
  private readonly dbPath: string;

  constructor(opts: ScoreboardOptions = {}) {
    this.dbPath = opts.dbPath ?? join(homedir(), '.claude', 'data', 'dashboard.db');
  }

  /** Get proposal counts for last 7 days. */
  weeklyStats(): ProposalStats {
    const now = new Date();
    const week = Math.ceil((now.getDate() - now.getDay() + 1) / 7);
    const period = `${now.getFullYear()}-W${String(week).padStart(2, '0')}`;

    if (!existsSync(this.dbPath)) {
      return { period, proposalsGenerated: 0, proposalsAccepted: 0, proposalsRejected: 0, acceptanceRate: 0 };
    }

    const fromTs = Date.now() - 7 * 24 * 60 * 60 * 1000;
    const base = `FROM evolution_proposals WHERE created_at >= ${fromTs}`;
    const proposalsGenerated = this.int(`SELECT COUNT(*) ${base};`);
    const proposalsAccepted = this.int(`SELECT COUNT(*) ${base} AND status='Applied';`);
    const proposalsRejected = this.int(`SELECT COUNT(*) ${base} AND status='Rejected';`);
    const acceptanceRate = proposalsGenerated > 0 ? proposalsAccepted / proposalsGenerated : 0;

    return { period, proposalsGenerated, proposalsAccepted, proposalsRejected, acceptanceRate };
  }

  /** Top-N proposals by delta score. */
  topProposals(n: number): ProposalScore[] {
    if (!existsSync(this.dbPath) || n <= 0) return [];

    const output = sqlite3Q(
      this.dbPath,
      `SELECT proposal_id, title, mode, completed_at, CAST(json_extract(result_json,'$.deltaScore') AS REAL) AS d ` +
        `FROM evolution_experiments WHERE status='completed' AND d IS NOT NULL ORDER BY d DESC LIMIT ${Math.floor(n)};`,
    );

    if (!output) return [];
    return output.split('\n').filter(Boolean).map((line) => {
      const [proposalId, title, mode, completedAt, deltaScore] = line.split('|');
      return { proposalId: proposalId ?? '', title: title ?? '', mode: mode ?? '', completedAt: parseInt(completedAt ?? '0', 10), deltaScore: parseFloat(deltaScore ?? '0') };
    });
  }

  private int(sql: string): number {
    return parseInt(sqlite3Q(this.dbPath, sql), 10) || 0;
  }
}
