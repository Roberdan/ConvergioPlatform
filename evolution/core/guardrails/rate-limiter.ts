/**
 * RateLimiter — enforces proposal cadence and token budget limits.
 *
 * Guards against runaway generation during autonomous operation.
 * All counts are in-memory; reset by calling resetCounts().
 */

export interface RateLimiterOptions {
  maxProposalsPerDay: number;
  maxProposalsPerWeek: number;
  /** Maximum USD token budget per evaluation cycle. */
  dailyTokenBudgetUsd: number;
}

export interface RateLimiterCounts {
  proposalsToday: number;
  proposalsThisWeek: number;
  tokenSpendUsd: number;
}

export class RateLimiter {
  private readonly options: RateLimiterOptions;
  private counts: RateLimiterCounts = {
    proposalsToday: 0,
    proposalsThisWeek: 0,
    tokenSpendUsd: 0,
  };

  constructor(options: RateLimiterOptions) {
    this.options = options;
  }

  /**
   * Returns false when the daily token budget has been exceeded.
   * The engine must not issue further LLM calls when this returns false.
   */
  checkTokenBudget(currentSpendUsd: number): boolean {
    return currentSpendUsd < this.options.dailyTokenBudgetUsd;
  }

  /** Returns true if another proposal may be generated today. */
  canGenerateProposal(): boolean {
    return (
      this.counts.proposalsToday < this.options.maxProposalsPerDay &&
      this.counts.proposalsThisWeek < this.options.maxProposalsPerWeek
    );
  }

  /** Records that a proposal was generated; increments counters. */
  recordProposal(): void {
    this.counts.proposalsToday += 1;
    this.counts.proposalsThisWeek += 1;
  }

  /** Records incremental token spend; call after each LLM invocation. */
  recordTokenSpend(usd: number): void {
    this.counts.tokenSpendUsd += usd;
  }

  /** Snapshot of current counters (read-only). */
  getCounts(): Readonly<RateLimiterCounts> {
    return { ...this.counts };
  }

  /** Resets daily counters. Call at midnight UTC. */
  resetDailyCounters(): void {
    this.counts.proposalsToday = 0;
    this.counts.tokenSpendUsd = 0;
  }

  /** Resets weekly counters. Call at 00:00 Monday UTC. */
  resetWeeklyCounters(): void {
    this.counts.proposalsThisWeek = 0;
  }

  /** Resets all counters (for testing). */
  resetCounts(): void {
    this.counts = { proposalsToday: 0, proposalsThisWeek: 0, tokenSpendUsd: 0 };
  }
}
