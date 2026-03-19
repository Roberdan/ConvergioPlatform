import { homedir } from 'os';
import { join } from 'path';
import { spawnSync } from 'child_process';
import type { Metric, MetricCollector } from '../../core/types/index.js';

export interface AgentMetricCollectorOptions {
  dbPath?: string;
  windowMs?: number;
}

export class AgentMetricCollector implements MetricCollector {
  readonly id = 'agent-telemetry';
  readonly families = ['Agent'] as const;

  private readonly dbPath: string;
  private readonly windowMs: number;

  constructor(options: AgentMetricCollectorOptions = {}) {
    this.dbPath = options.dbPath ?? join(homedir(), '.claude', 'data', 'dashboard.db');
    this.windowMs = options.windowMs ?? 5 * 60 * 1000;
  }

  async collect(): Promise<Metric[]> {
    const now = Date.now();
    const windowSeconds = Math.max(Math.floor(this.windowMs / 1000), 1);
    const metrics: Metric[] = [];

    const totals = this.queryRow(
      `SELECT
        COALESCE(SUM(input_tokens),0),
        COALESCE(SUM(output_tokens),0),
        COALESCE(SUM(cost_usd),0)
       FROM token_usage
       WHERE datetime(created_at) >= datetime('now','-${windowSeconds} seconds');`,
    );

    const inputTokens = parseNumber(totals[0]);
    const outputTokens = parseNumber(totals[1]);
    const totalCost = parseNumber(totals[2]);

    metrics.push(buildMetric('agent.tokens.input', inputTokens, now));
    metrics.push(buildMetric('agent.tokens.output', outputTokens, now));
    metrics.push(buildMetric('agent.cost.usd', totalCost, now));

    const sessions = parseNumber(
      this.queryScalar(
        `SELECT COUNT(DISTINCT COALESCE(task_id, created_at))
         FROM token_usage
         WHERE datetime(created_at) >= datetime('now','-${windowSeconds} seconds');`,
      ),
    );
    metrics.push(buildMetric('agent.session.count', sessions, now));

    const completionRate = parseNumber(
      this.queryScalar(
        "SELECT CASE WHEN COUNT(*)=0 THEN 0 ELSE CAST(SUM(CASE WHEN status='done' THEN 1 ELSE 0 END) AS REAL)/COUNT(*) END FROM tasks;",
      ),
    );
    metrics.push(buildMetric('agent.task.completion_rate', completionRate, now));

    const topModelRow = this.queryRow(
      `SELECT model, COUNT(*)
       FROM token_usage
       WHERE datetime(created_at) >= datetime('now','-${windowSeconds} seconds')
       GROUP BY model
       ORDER BY COUNT(*) DESC
       LIMIT 1;`,
    );
    const topModel = (topModelRow[0] || 'unknown').trim();
    metrics.push(buildMetric('agent.model.top', 1, now, { model: topModel }));

    const activePlans = parseNumber(
      this.queryScalar("SELECT COUNT(*) FROM plans WHERE status IN ('todo','doing');"),
    );
    metrics.push(buildMetric('agent.plan.active_count', activePlans, now));

    return metrics;
  }

  private queryScalar(sql: string): string {
    return this.queryRow(sql)[0] ?? '0';
  }

  private queryRow(sql: string): string[] {
    const result = spawnSync('sqlite3', ['-separator', '|', this.dbPath, sql], {
      encoding: 'utf8',
    });
    if (result.status !== 0) {
      return [];
    }
    const line = (result.stdout ?? '').trim().split('\n')[0] ?? '';
    if (!line) {
      return [];
    }
    return line.split('|');
  }
}

function buildMetric(name: string, value: number, timestamp: number, labels: Record<string, string> = {}): Metric {
  return {
    name,
    value,
    timestamp,
    labels,
    family: 'Agent',
  };
}

function parseNumber(value: string | undefined): number {
  const parsed = Number(value ?? 0);
  return Number.isFinite(parsed) ? parsed : 0;
}

