/**
 * RoiTracker — computes weekly ROI summary from evolution experiment data.
 *
 * Reads completed experiments from the evolution_experiments table.
 * estimated_savings_usd = experiments_successful * 0.10 (placeholder formula).
 * This is a conservative placeholder; replace with actual cost-model data.
 */

import { spawnSync } from 'child_process';
import { existsSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';

/** Weekly ROI aggregation for the Evolution Engine. */
export interface RoiSummary {
  /** ISO week label, e.g. "2024-W24" */
  period: string;
  /** Number of proposals created in the period */
  proposalsGenerated: number;
  /** Number of experiments completed in the period */
  experimentsRun: number;
  /** Experiments that ended with status='rolledback' */
  rollbacks: number;
  /** Sum of result_json.deltaScore across completed experiments */
  netDeltaScore: number;
  /**
   * Placeholder savings estimate.
   * Formula: (experimentsRun - rollbacks) * 0.10 USD
   * Replace with real cost-model integration.
   */
  estimatedSavingsUsd: number;
}

export interface RoiTrackerOptions {
  /** Path to the dashboard/evolution SQLite database. */
  dbPath?: string;
}

function sqlite3Query(dbPath: string, sql: string): string {
  const r = spawnSync('sqlite3', [dbPath, sql], { encoding: 'utf-8' });
  return r.status === 0 ? (r.stdout ?? '').trim() : '';
}

function isoWeekLabel(date: Date): string {
  const jan4 = new Date(date.getFullYear(), 0, 4);
  const startOfWeek1 = new Date(jan4);
  startOfWeek1.setDate(jan4.getDate() - jan4.getDay() + 1);
  const diffMs = date.getTime() - startOfWeek1.getTime();
  const week = Math.ceil((diffMs / 86_400_000 + 1) / 7);
  return `${date.getFullYear()}-W${String(week).padStart(2, '0')}`;
}

export class RoiTracker {
  private readonly dbPath: string;

  constructor(options: RoiTrackerOptions = {}) {
    this.dbPath =
      options.dbPath ?? join(homedir(), '.claude', 'data', 'dashboard.db');
  }

  /** Compute weekly ROI summary for the last 7 days. */
  computeWeekly(): RoiSummary {
    const period = isoWeekLabel(new Date());

    if (!existsSync(this.dbPath)) {
      return this.emptyWeek(period);
    }

    const sevenDaysAgo = Date.now() - 7 * 24 * 60 * 60 * 1000;
    const fromTs = Math.floor(sevenDaysAgo);

    const experimentsRun = this.queryInt(
      `SELECT COUNT(*) FROM evolution_experiments WHERE status='completed' AND completed_at >= ${fromTs};`,
    );

    const rollbacks = this.queryInt(
      `SELECT COUNT(*) FROM evolution_experiments WHERE status='rolledback' AND completed_at >= ${fromTs};`,
    );

    const proposalsGenerated = this.queryInt(
      `SELECT COUNT(*) FROM evolution_proposals WHERE created_at >= ${fromTs};`,
    );

    const rawDelta = sqlite3Query(
      this.dbPath,
      `SELECT COALESCE(SUM(CAST(json_extract(result_json,'$.deltaScore') AS REAL)),0) ` +
        `FROM evolution_experiments WHERE status='completed' AND completed_at >= ${fromTs};`,
    );
    const netDeltaScore = parseFloat(rawDelta) || 0;

    const successful = Math.max(0, experimentsRun - rollbacks);
    const estimatedSavingsUsd = successful * 0.1;

    return {
      period,
      proposalsGenerated,
      experimentsRun,
      rollbacks,
      netDeltaScore,
      estimatedSavingsUsd,
    };
  }

  private queryInt(sql: string): number {
    return parseInt(sqlite3Query(this.dbPath, sql), 10) || 0;
  }

  private emptyWeek(period: string): RoiSummary {
    return {
      period,
      proposalsGenerated: 0,
      experimentsRun: 0,
      rollbacks: 0,
      netDeltaScore: 0,
      estimatedSavingsUsd: 0,
    };
  }
}
