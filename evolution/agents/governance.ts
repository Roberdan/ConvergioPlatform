import type { Proposal } from '../core/types/index.js';

export interface GovernanceResult {
  approved: boolean;
  prRequired: boolean;
  approvers: string[];
  reason: string;
}

export class AgentGovernance {
  validateProposal(proposal: Proposal, requiredApprovers: string[]): GovernanceResult {
    const prRequired = true; // prOnly governance for all agent changes
    const delta = inferCostDelta(proposal);
    const requiresHumanGate = delta > 0.2;

    if (requiresHumanGate && requiredApprovers.length === 0) {
      return {
        approved: false,
        prRequired,
        approvers: [],
        reason: 'ApprovalGate: costPerTask delta above 20%; human review required before draftPR can merge.',
      };
    }

    return {
      approved: true,
      prRequired,
      approvers: requiredApprovers,
      reason: 'PR required for agent changes; direct push is blocked by governance policy.',
    };
  }
}

function inferCostDelta(proposal: Proposal): number {
  if (proposal.expectedDelta) {
    return Math.abs(proposal.expectedDelta.max);
  }
  const text = proposal.estimatedGain;
  const percentMatch = text.match(/(-?\d+(?:\.\d+)?)%/);
  if (!percentMatch) {
    return 0;
  }
  return Math.abs(Number(percentMatch[1]) / 100);
}
