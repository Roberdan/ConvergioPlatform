const DOMAIN_TAGS: Record<string, string[]> = {
  latency: ['performance', 'http'],
  bundle: ['frontend', 'webpack'],
  agent: ['llm', 'cost', 'tokens'],
};

export type HypothesisStatus = 'HYPOTHESIS';

export interface HypothesisMetadata {
  status: HypothesisStatus;
  sourceUrl: string;
  retrievalDate: string;
  confidence: number;
}

export class HypothesisTagger {
  tag(text: string, domain: string): string[] {
    const normalizedDomain = domain.toLowerCase().trim();
    const normalizedText = text.toLowerCase();

    for (const [keyword, tags] of Object.entries(DOMAIN_TAGS)) {
      if (normalizedDomain.includes(keyword) || normalizedText.includes(keyword)) {
        return [...tags];
      }
    }

    return normalizedDomain ? [normalizedDomain] : ['general'];
  }

  createMetadata(sourceUrl: string, confidence = 0.6): HypothesisMetadata {
    return {
      status: 'HYPOTHESIS',
      sourceUrl,
      retrievalDate: new Date().toISOString(),
      confidence,
    };
  }
}
