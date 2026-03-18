import type { Proposal } from '../core/types/index.js';
import type { AgentProfile } from './profiler.js';

interface ProposalWithConfidence extends Proposal {
  confidence: number;
}

const LATEST_MODEL_VERSION: Record<string, string> = {
  'claude-opus': '4.6',
  'claude-sonnet': '4.6',
  'claude-haiku': '4.5',
  'gpt-5': '5.4',
  'gpt-5.3-codex': '5.3-codex',
};

export class AgentProposalGenerator {
  generate(profile: AgentProfile): Proposal[] {
    const proposals: ProposalWithConfidence[] = [];
    if (profile.costPerTask > 0.5) proposals.push(this.build('Route simple tasks to cheaper model', profile, 0.8));
    if (profile.completionRate < 0.7) proposals.push(this.build('Review task decomposition for agent', profile, 0.7));
    if (profile.avgTokensPerTask > 50_000) proposals.push(this.build('Add context compression', profile, 0.75));
    if (!isLatestModel(profile.topModel)) proposals.push(this.build('Upgrade to latest model version', profile, 0.6));
    return proposals;
  }

  private build(hypothesis: string, profile: AgentProfile, confidence: number): ProposalWithConfidence {
    const suffix = Math.floor(Math.random() * 10_000).toString().padStart(4, '0');
    return {
      id: `EVO-${new Date().toISOString().slice(0, 10).replaceAll('-', '')}-${suffix}`,
      hypothesis,
      targetMetric: 'agent.optimization.score',
      expectedDelta: { min: 0.03, max: 0.2 },
      successCriteria: `Improve ${profile.agentName}; confidence=${confidence}`,
      failureCriteria: 'Task quality drops or completion_rate regresses',
      blastRadius: 'SingleRepo',
      sourceType: 'Internal',
      status: 'Draft',
      confidence,
    };
  }
}

function isLatestModel(model: string): boolean {
  const normalized = model.toLowerCase();
  for (const [family, latest] of Object.entries(LATEST_MODEL_VERSION)) {
    if (normalized.includes(family)) return normalized.includes(latest.toLowerCase());
  }
  return true;
}
