import { spawnSync } from 'node:child_process';
import type {
  BlastRadius,
  EvaluationResult,
  EvolutionConfig,
  OptimizationOpportunity,
  Proposal,
} from '../types/index.js';

export interface ProposalGeneratorOptions {
  maxProposalsPerCycle?: number;
  dbPath?: string;
}

const BLAST_RADIUS_WEIGHT: Record<BlastRadius, number> = {
  SingleFile: 1,
  SingleRepo: 0.8,
  MultiRepo: 0.5,
  Ecosystem: 0.3,
};

export class ProposalGenerator {
  private readonly maxProposalsPerCycle: number;
  private readonly dbPath: string;

  constructor(options: ProposalGeneratorOptions = {}) {
    this.maxProposalsPerCycle = options.maxProposalsPerCycle ?? 5;
    this.dbPath = options.dbPath ?? '.evolution-outcomes.db';
    this.ensureOutcomesSchema();
  }

  static fromConfig(config: EvolutionConfig): ProposalGenerator {
    return new ProposalGenerator({
      maxProposalsPerCycle: config.maxProposalsPerCycle ?? 5,
    });
  }

  generate(evaluations: EvaluationResult[], adapters: string[]): Proposal[] {
    const ranked: Array<{ proposal: Proposal; confidence: number }> = [];

    for (const evaluation of evaluations) {
      for (const opportunity of evaluation.opportunities) {
        if (this.wasRecentlyProposed(opportunity.title, 7)) {
          continue;
        }

        const confidence = this.scoreProposal(opportunity, evaluation.score);
        ranked.push({
          confidence,
          proposal: this.toProposal(opportunity, evaluation.score, confidence, adapters),
        });
      }
    }

    return ranked
      .sort((left, right) => right.confidence - left.confidence)
      .slice(0, this.maxProposalsPerCycle)
      .map((entry) => entry.proposal);
  }

  scoreProposal(opportunity: OptimizationOpportunity, domainScore: number): number {
    const base = Math.max(0, Math.min(1, (100 - domainScore) / 100));
    const weight = BLAST_RADIUS_WEIGHT[opportunity.suggestedBlastRadius] ?? 0.3;
    return Number((base * weight).toFixed(3));
  }

  private toProposal(
    opportunity: OptimizationOpportunity,
    domainScore: number,
    confidence: number,
    adapters: string[],
  ): Proposal {
    const id = `EVO-${new Date().toISOString().slice(0, 10).replace(/-/g, '')}-${Math.random()
      .toString(36)
      .slice(2, 6)
      .toUpperCase()}`;

    return {
      id,
      title: opportunity.title,
      description: opportunity.description,
      blastRadius: opportunity.suggestedBlastRadius,
      sourceType: 'Internal',
      status: 'Draft',
      targetAdapter: adapters[0] ?? 'template',
      failureCriteria: 'Any guarded metric regresses by more than 2%',
      rollbackStrategy: 'Auto-revert canary and restore previous baseline',
      estimatedGain: opportunity.estimatedGain,
      confidence,
      createdAt: Date.now(),
      hypothesisRef: undefined,
      hypothesis: opportunity.description,
      targetMetric: `${opportunity.domain}.score`,
      expectedDelta: { min: -0.05, max: -0.2 },
      successCriteria: `${opportunity.domain} score improves by >= ${Math.max(5, 100 - domainScore)}%`,
      appliedRef: undefined,
    };
  }

  private wasRecentlyProposed(title: string, days: number): boolean {
    const cutoff = Date.now() - days * 24 * 60 * 60 * 1000;
    const sql = `SELECT COUNT(*) FROM evolution_outcomes WHERE proposal_title = '${escapeSql(
      title,
    )}' AND created_at >= ${cutoff};`;
    try {
      const count = this.queryOne(sql);
      return count > 0;
    } catch {
      return false;
    }
  }

  private ensureOutcomesSchema(): void {
    this.exec(`CREATE TABLE IF NOT EXISTS evolution_outcomes (
      id INTEGER PRIMARY KEY,
      proposal_id TEXT NOT NULL,
      proposal_title TEXT,
      domain TEXT,
      before_score REAL,
      after_score REAL,
      delta_score REAL,
      improved INTEGER,
      created_at INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_evolution_outcomes_title ON evolution_outcomes(proposal_title, created_at DESC);`);
  }

  private exec(sql: string): void {
    const result = spawnSync('sqlite3', [this.dbPath, sql], { encoding: 'utf8' });
    if (result.status !== 0) {
      throw new Error(result.stderr || 'sqlite3 execution failed');
    }
  }

  private queryOne(sql: string): number {
    const result = spawnSync('sqlite3', [this.dbPath, sql], { encoding: 'utf8' });
    if (result.status !== 0) {
      throw new Error(result.stderr || 'sqlite3 query failed');
    }
    return Number(result.stdout.trim() || 0);
  }
}

function escapeSql(value: string): string {
  return value.replace(/'/g, "''");
}
