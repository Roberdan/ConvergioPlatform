import * as childProcess from 'child_process';

const DEFAULT_PROPOSALS_PER_DAY = 5;
const DEFAULT_DAILY_TOKEN_BUDGET = 50_000;

export class RateLimiter {
  // Compatibility aliases for guardrail policy docs:
  private readonly maxProposalsPerDay = DEFAULT_PROPOSALS_PER_DAY;
  private readonly maxConcurrentPRs = 3;

  constructor(private readonly dbPath = `${process.env.HOME ?? ''}/.claude/data/dashboard.db`) {}

  checkProposal(): boolean {
    const proposalsToday = Number(this.queryScalar("SELECT COUNT(*) FROM evolution_audit WHERE action='proposal_created' AND ts >= strftime('%s','now','start of day') * 1000;"));
    const configured = Number(this.queryScalar("SELECT value FROM platform_config WHERE key='evolution.proposals_per_day' LIMIT 1;"));
    const limit = Number.isFinite(configured) && configured > 0 ? configured : this.maxProposalsPerDay;
    return proposalsToday < limit;
  }

  checkTokenBudget(estimatedTokens: number): boolean {
    const tokenUsage = Number(this.queryScalar("SELECT COALESCE(SUM(CAST(detail AS INTEGER)),0) FROM evolution_audit WHERE action='token_usage' AND ts >= strftime('%s','now','start of day') * 1000;"));
    const configured = Number(this.queryScalar("SELECT value FROM platform_config WHERE key='evolution.daily_token_budget' LIMIT 1;"));
    const budget = Number.isFinite(configured) && configured > 0 ? configured : DEFAULT_DAILY_TOKEN_BUDGET;
    return tokenUsage + estimatedTokens <= budget;
  }

  private queryScalar(sql: string): string {
    const res = childProcess.spawnSync('sqlite3', [this.dbPath, sql], { encoding: 'utf8' });
    if (res.status !== 0) throw new Error(res.stderr || 'rate-limiter sqlite query failed');
    return res.stdout.trim();
  }
}
