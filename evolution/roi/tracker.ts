/**
 * RoiTracker — computes ROI metrics for the Evolution Engine.
 *
 * Consolidated from evolution/roi/tracker.ts and evolution/reporting/roi-tracker.ts.
 * Uses daemon API (http://localhost:8420) instead of direct sqlite3 access.
 */

/** Unified ROI summary covering all consumers (roi/ and reporting/). */
export interface RoiSummary {
  /** ISO week label, e.g. "2024-W24" */
  period: string;
  /** Number of proposals created in the period */
  proposalsGenerated: number;
  /** Number of experiments completed */
  experimentsRun: number;
  /** Experiments rolled back */
  rollbacks: number;
  /** Sum of deltaScore improvements from completed experiments */
  netDeltaScore: number;
  /** Success rate as a percentage (0–100) */
  successRate: number;
  /** Estimated USD savings: successful experiments * 0.10 */
  estimatedSavingsUsd: number;
}

export interface RoiTrackerOptions {
  /** Base URL for the daemon API. Defaults to http://localhost:8420. */
  daemonUrl?: string;
}

function isoWeekLabel(date: Date): string {
  const jan4 = new Date(date.getFullYear(), 0, 4);
  const startOfWeek1 = new Date(jan4);
  startOfWeek1.setDate(jan4.getDate() - jan4.getDay() + 1);
  const week = Math.ceil((date.getTime() - startOfWeek1.getTime()) / 86_400_000 / 7 + 1);
  return `${date.getFullYear()}-W${String(week).padStart(2, '0')}`;
}

export class RoiTracker {
  private readonly daemonUrl: string;

  constructor(opts: RoiTrackerOptions = {}) {
    this.daemonUrl = opts.daemonUrl ?? 'http://localhost:8420';
  }

  /** Compute weekly ROI summary via daemon API. Returns empty summary if daemon is unreachable. */
  async computeWeekly(): Promise<RoiSummary> {
    const period = isoWeekLabel(new Date());
    try {
      const [roiResp, proposalsResp] = await Promise.all([
        fetch(`${this.daemonUrl}/api/evolution/roi`),
        fetch(`${this.daemonUrl}/api/evolution/proposals`),
      ]);
      if (!roiResp.ok) return this.empty(period);

      const roi = (await roiResp.json()) as {
        experimentsRun?: number;
        rollbacks?: number;
        successRate?: number;
      };
      const experimentsRun = roi.experimentsRun ?? 0;
      const rollbacks = roi.rollbacks ?? 0;
      const successRate = roi.successRate ?? 0;
      const successful = Math.max(0, experimentsRun - rollbacks);

      let proposalsGenerated = 0;
      if (proposalsResp.ok) {
        const proposals = (await proposalsResp.json()) as unknown[];
        proposalsGenerated = Array.isArray(proposals) ? proposals.length : 0;
      }

      return {
        period,
        proposalsGenerated,
        experimentsRun,
        rollbacks,
        netDeltaScore: successRate,
        successRate,
        estimatedSavingsUsd: successful * 0.1,
      };
    } catch {
      return this.empty(period);
    }
  }

  private empty(period: string): RoiSummary {
    return {
      period,
      proposalsGenerated: 0,
      experimentsRun: 0,
      rollbacks: 0,
      netDeltaScore: 0,
      successRate: 0,
      estimatedSavingsUsd: 0,
    };
  }
}
