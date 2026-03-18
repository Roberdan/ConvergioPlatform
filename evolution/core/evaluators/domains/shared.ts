import type { Metric, OptimizationOpportunity, BlastRadius } from '../../types/index.js';

export function latestValue(metrics: Metric[], name: string): number | null {
  const matched = metrics
    .filter((metric) => metric.name === name)
    .sort((left, right) => right.timestamp - left.timestamp);
  return matched[0]?.value ?? null;
}

export function opportunities(
  domain: string,
  titles: string[],
  estimatedGain: string,
  suggestedBlastRadius: BlastRadius = 'SingleRepo',
): OptimizationOpportunity[] {
  return titles.map((title) => ({
    title,
    description: `${title} to improve ${domain}`,
    estimatedGain,
    domain,
    suggestedBlastRadius,
  }));
}
