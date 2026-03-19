import type { Proposal } from './pr-only-enforcer.js';

export interface SafetyReport {
  safe: boolean;
  warnings: string[];
  blockers: string[];
}

export class SafetyValidator {
  private readonly prOnlyMode = true;
  private readonly rateLimit = 'enabled';

  validate(proposal: Proposal): SafetyReport {
    const warnings: string[] = [];
    const blockers: string[] = [];

    if (!proposal.rollbackStrategy.trim()) {
      blockers.push('rollback strategy is required');
    }
    if (!proposal.failureCriteria.trim()) {
      blockers.push('failure criteria is required');
    }
    if (proposal.blastRadius === 'Ecosystem') {
      blockers.push('blastRadius Ecosystem is blocked (APPROVAL_REQUIRED)');
    }
    if (proposal.confidence <= 0.3) {
      warnings.push('confidence is low, requires human review');
    }

    if (this.prOnlyMode && this.rateLimit === 'enabled' && proposal.blastRadius === 'MultiRepo') {
      warnings.push('PR-only mode active for multi-repo changes');
    }

    return {
      safe: blockers.length === 0,
      warnings,
      blockers,
    };
  }
}
