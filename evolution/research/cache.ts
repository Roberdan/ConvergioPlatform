/**
 * research cache with TTL and dedup semantics by query hash.
 */
export interface CachedResearchPayload {
  queryHash: string;
  resultJson: string;
  cacheCreatedAt: number;
  expiresAt: number;
}

export function defaultTTLms(): number {
  // Default cache TTL = 7d
  return 7 * 24 * 60 * 60 * 1000;
}

export function dedupKey(query: string): string {
  return query.trim().toLowerCase();
}
