/**
 * Scoreboard — ranks proposals by experiment delta score.
 *
 * Reads evolution_experiments ordered by result_json.deltaScore DESC.
 * Used to surface the highest-impact proposals for the dashboard.
 */

import { spawnSync } from 'child_process';
import { existsSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';

/** A ranked proposal entry for the scoreboard. */
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

function sqlite3Query(dbPath: string, sql: string): string {
  const r = spawnSync('sqlite3', [dbPath, sql], { encoding: 'utf-8' });
  return r.status === 0 ? (r.stdout ?? '').trim() : '';
}

export class Scoreboard {
  private readonly dbPath: string;

  constructor(options: ScoreboardOptions = {}) {
    this.dbPath =
      options.dbPath ?? join(homedir(), '.claude', 'data', 'dashboard.db');
  }

  /**
   * Returns the top-N proposals ranked by experiment delta score (highest first).
   * Returns an empty array if the database is unavailable.
   */
  topProposals(n: number): ProposalScore[] {
    if (!existsSync(this.dbPath) || n <= 0) {
      return [];
    }

    const sql =
      `SELECT proposal_id, title, mode, completed_at, ` +
      `CAST(json_extract(result_json,'$.deltaScore') AS REAL) AS delta ` +
      `FROM evolution_experiments ` +
      `WHERE status='completed' AND delta IS NOT NULL ` +
      `ORDER BY delta DESC LIMIT ${Math.floor(n)};`;

    const output = sqlite3Query(this.dbPath, sql);
    if (!output) {
      return [];
    }

    return output
      .split('\n')
      .filter(Boolean)
      .map((line) => {
        const [proposalId, title, mode, completedAt, deltaScore] =
          line.split('|');
        return {
          proposalId: proposalId ?? '',
          title: title ?? '',
          mode: mode ?? '',
          completedAt: parseInt(completedAt ?? '0', 10),
          deltaScore: parseFloat(deltaScore ?? '0'),
        };
      });
  }
}
