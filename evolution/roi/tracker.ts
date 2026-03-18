/**
 * RoiTracker — computes ROI metrics for the Evolution Engine.
 *
 * Tracks improvement gains vs system costs to calculate net ROI.
 * estimated formula: netROI = improvementGains - systemCost (placeholder).
 */

import { spawnSync } from 'child_process';
import { existsSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';

/** ROI summary with gains and costs. */
export interface RoiSummary {
  /** ISO week label, e.g. "2024-W24" */
  period: string;
  /** Number of proposals created in the period */
  proposalsGenerated: number;
  /** Number of experiments completed */
  experimentsRun: number;
  /** Experiments rolled back */
  rollbacks: number;
  /** Sum of deltaScore improvements: positive = gain */
  improvementGains: number;
  /** Estimated cost in USD: token spend + rollback overhead */
  systemCost: number;
  /** Net ROI: improvementGains - systemCost (USD-normalised placeholder) */
  netROI: number;
  /** Estimated USD savings: successful experiments * 0.10 */
  estimatedSavingsUsd: number;
}

export interface RoiTrackerOptions {
  dbPath?: string;
}

function sqlite3Q(dbPath: string, sql: string): string {
  const r = spawnSync('sqlite3', [dbPath, sql], { encoding: 'utf-8' });
  return r.status === 0 ? (r.stdout ?? '').trim() : '';
}

function isoWeek(date: Date): string {
  const jan4 = new Date(date.getFullYear(), 0, 4);
  const sw = new Date(jan4);
  sw.setDate(jan4.getDate() - jan4.getDay() + 1);
  const week = Math.ceil((date.getTime() - sw.getTime()) / 86_400_000 / 7 + 1);
  return `${date.getFullYear()}-W${String(week).padStart(2, '0')}`;
}

export class RoiTracker {
  private readonly dbPath: string;

  constructor(opts: RoiTrackerOptions = {}) {
    this.dbPath = opts.dbPath ?? join(homedir(), '.claude', 'data', 'dashboard.db');
  }

  /** Compute weekly ROI summary for the last 7 days. */
  computeWeekly(): RoiSummary {
    const period = isoWeek(new Date());
    if (!existsSync(this.dbPath)) return this.empty(period);

    const fromTs = Date.now() - 7 * 24 * 60 * 60 * 1000;
    const experimentsRun = this.int(
      `SELECT COUNT(*) FROM evolution_experiments WHERE status='completed' AND completed_at >= ${fromTs};`,
    );
    const rollbacks = this.int(
      `SELECT COUNT(*) FROM evolution_experiments WHERE status='rolledback' AND completed_at >= ${fromTs};`,
    );
    const proposalsGenerated = this.int(
      `SELECT COUNT(*) FROM evolution_proposals WHERE created_at >= ${fromTs};`,
    );
    const raw = sqlite3Q(
      this.dbPath,
      `SELECT COALESCE(SUM(CAST(json_extract(result_json,'$.deltaScore') AS REAL)),0) ` +
        `FROM evolution_experiments WHERE status='completed' AND completed_at >= ${fromTs};`,
    );
    const improvementGains = parseFloat(raw) || 0;
    const successful = Math.max(0, experimentsRun - rollbacks);
    const estimatedSavingsUsd = successful * 0.1;
    const systemCost = rollbacks * 0.05;
    const netROI = improvementGains - systemCost;

    return { period, proposalsGenerated, experimentsRun, rollbacks, improvementGains, systemCost, netROI, estimatedSavingsUsd };
  }

  private int(sql: string): number {
    return parseInt(sqlite3Q(this.dbPath, sql), 10) || 0;
  }

  private empty(period: string): RoiSummary {
    return { period, proposalsGenerated: 0, experimentsRun: 0, rollbacks: 0, improvementGains: 0, systemCost: 0, netROI: 0, estimatedSavingsUsd: 0 };
  }
}
