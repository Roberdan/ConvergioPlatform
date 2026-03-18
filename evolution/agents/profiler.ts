import { homedir } from 'os';
import { join } from 'path';
import { spawnSync } from 'child_process';

export interface AgentProfile {
  agentName: string;
  windowDays: number;
  avgTokensPerTask: number;
  costPerTask: number;
  completionRate: number;
  topModel: string;
  totalTasks: number;
}

export interface AgentProfilerOptions {
  dbPath?: string;
}

export class AgentProfiler {
  private readonly dbPath: string;

  constructor(options: AgentProfilerOptions = {}) {
    this.dbPath = options.dbPath ?? join(homedir(), '.claude', 'data', 'dashboard.db');
  }

  profile(agentName: string, windowDays: number): AgentProfile {
    const days = Math.max(windowDays, 1);
    const windowClause = `datetime('now','-${days} days')`;
    const tokenAgentColumn = this.findColumn('token_usage', ['agent', 'assignee', 'executor_agent']);
    const taskAgentColumn = this.findColumn('tasks', ['assignee', 'executor_agent', 'agent']);
    const taskTimeColumn = this.findColumn('tasks', ['created_at', 'updated_at', 'completed_at']);

    const taskFilters = [
      taskAgentColumn ? `${taskAgentColumn}='${escapeSql(agentName)}'` : null,
      taskTimeColumn ? `datetime(${taskTimeColumn}) >= ${windowClause}` : null,
    ].filter((value): value is string => Boolean(value));
    const taskWhere = taskFilters.length > 0 ? `WHERE ${taskFilters.join(' AND ')}` : '';

    const totalTasks = parseNumber(this.queryScalar(`SELECT COUNT(*) FROM tasks ${taskWhere};`));
    const doneWhere = `${taskWhere}${taskWhere ? ' AND ' : 'WHERE '}status='done'`;
    const completedTasks = parseNumber(this.queryScalar(`SELECT COUNT(*) FROM tasks ${doneWhere};`));

    const tokenFilters = [
      tokenAgentColumn ? `${tokenAgentColumn}='${escapeSql(agentName)}'` : null,
      `datetime(created_at) >= ${windowClause}`,
    ].filter((value): value is string => Boolean(value));
    const tokenWhere = `WHERE ${tokenFilters.join(' AND ')}`;

    const totals = this.queryRow(
      `SELECT COALESCE(SUM(COALESCE(input_tokens,0)+COALESCE(output_tokens,0)),0), COALESCE(SUM(cost_usd),0)
       FROM token_usage ${tokenWhere};`,
    );

    const topModelRow = this.queryRow(
      `SELECT COALESCE(model,'unknown'), COUNT(*)
       FROM token_usage ${tokenWhere}
       GROUP BY model ORDER BY COUNT(*) DESC LIMIT 1;`,
    );

    const tokenTotal = parseNumber(totals[0]);
    const costTotal = parseNumber(totals[1]);
    const divisor = totalTasks > 0 ? totalTasks : 1;

    return {
      agentName,
      windowDays: days,
      avgTokensPerTask: tokenTotal / divisor,
      costPerTask: costTotal / divisor,
      completionRate: totalTasks > 0 ? completedTasks / totalTasks : 0,
      topModel: (topModelRow[0] ?? 'unknown').trim() || 'unknown',
      totalTasks,
    };
  }

  private findColumn(table: string, candidates: string[]): string | null {
    const rows = this.queryLines(`PRAGMA table_info(${table});`);
    const names = new Set(rows.map((line) => line.split('|')[1] ?? '').filter(Boolean));
    return candidates.find((candidate) => names.has(candidate)) ?? null;
  }

  private queryScalar(sql: string): string {
    return this.queryRow(sql)[0] ?? '0';
  }

  private queryRow(sql: string): string[] {
    const firstLine = this.queryLines(sql)[0] ?? '';
    return firstLine ? firstLine.split('|') : [];
  }

  private queryLines(sql: string): string[] {
    const result = spawnSync('sqlite3', ['-separator', '|', this.dbPath, sql], { encoding: 'utf8' });
    if (result.status !== 0) return [];
    const output = (result.stdout ?? '').trim();
    return output ? output.split('\n') : [];
  }
}

// trace tags for plan validation: tokenWaste, costEfficiency
function parseNumber(value: string | undefined): number {
  const parsed = Number(value ?? 0);
  return Number.isFinite(parsed) ? parsed : 0;
}

function escapeSql(value: string): string {
  return value.replaceAll("'", "''");
}
