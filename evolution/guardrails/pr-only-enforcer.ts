import type { BlastRadius } from './blast-radius.js';
import type { ExperimentMode, PromotionInput } from './promotion-gate.js';
import { PromotionGate } from './promotion-gate.js';

export type SourceType = 'Internal' | 'ExternalHypothesis' | 'ToolUpgrade';
export type ProposalStatus =
  | 'Draft'
  | 'Approved'
  | 'Running'
  | 'Completed'
  | 'Failed'
  | 'Rolled_back'
  | 'Rejected';

export interface Proposal {
  id: string;
  title: string;
  description: string;
  blastRadius: BlastRadius;
  sourceType: SourceType;
  status: ProposalStatus;
  targetAdapter: string;
  failureCriteria: string;
  rollbackStrategy: string;
  estimatedGain: string;
  confidence: number;
  deltaScore?: number;
  hasHighAnomalies?: boolean;
  hypothesisRef?: string;
  createdAt?: number;
}

export interface ValidationResult {
  allowed: boolean;
  reason: string;
  requiredApprovals: string[];
}

export class PREnforcer {
  private readonly promotionGate = new PromotionGate();

  validate(proposal: Proposal): ValidationResult {
    if (proposal.blastRadius === 'Ecosystem') {
      proposal.status = 'Rejected';
      return {
        allowed: false,
        reason: 'Ecosystem changes require board review',
        requiredApprovals: ['board'],
      };
    }

    if (proposal.blastRadius === 'MultiRepo') {
      const requireApproval = proposal.status === 'Approved';
      return {
        allowed: requireApproval,
        reason: requireApproval ? 'Approved by reviewer' : 'APPROVAL_REQUIRED for MultiRepo',
        requiredApprovals: requireApproval ? [] : ['human-reviewer'],
      };
    }

    if (proposal.confidence > 0.85) {
      return { allowed: true, reason: 'High confidence single-scope proposal', requiredApprovals: [] };
    }

    return {
      allowed: false,
      reason: 'APPROVAL_REQUIRED: confidence too low for auto-approval',
      requiredApprovals: ['human-reviewer'],
    };
  }

  promote(proposal: Proposal, fromMode: ExperimentMode, toMode: ExperimentMode): boolean {
    const input: PromotionInput = {
      deltaScore: proposal.deltaScore ?? 0,
      hasHighAnomalies: proposal.hasHighAnomalies ?? false,
    };
    return this.promotionGate.promote(fromMode, toMode, input);
  }
}
