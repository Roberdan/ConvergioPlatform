export interface ReportInput {
  experimentId: string;
  mode: 'Shadow' | 'Canary' | 'Full';
  proposalTitle: string;
  durationMs: number;
  beforeAfter: {
    before: Array<{ name: string; value: number }>;
    after: Array<{ name: string; value: number }>;
  };
  confidenceInterval: [number, number];
  pValue: number;
  recommendation: 'Apply' | 'Reject' | 'ExtendCanary' | 'Inconclusive';
  anomaliesResolved: string[];
  deltaScore: number;
}

export class ReportGenerator {
  generate(input: ReportInput): string {
    const lines = [
      `# Experiment ${input.experimentId}`,
      `- mode: ${input.mode}`,
      `- proposal: ${input.proposalTitle}`,
      `- durationMs: ${input.durationMs}`,
      `- deltaScore: ${input.deltaScore.toFixed(4)}`,
      `- recommendation: ${input.recommendation}`,
      `- pValue: ${input.pValue.toFixed(4)}`,
      `- confidenceInterval: [${input.confidenceInterval[0].toFixed(4)}, ${input.confidenceInterval[1].toFixed(4)}]`,
      `- anomaliesResolved: ${input.anomaliesResolved.join(', ') || 'none'}`,
      '- beforeAfter:',
      `  - before: ${input.beforeAfter.before.map((item) => `${item.name}=${item.value}`).join(', ') || 'none'}`,
      `  - after: ${input.beforeAfter.after.map((item) => `${item.name}=${item.value}`).join(', ') || 'none'}`,
    ];
    return lines.slice(0, 30).join('\n');
  }
}
