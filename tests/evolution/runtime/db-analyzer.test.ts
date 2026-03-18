import { describe, expect, it } from 'vitest';
import { DbAnalyzer } from '../../../src/ts/evolution/runtime/db-analyzer.ts';

describe('DbAnalyzer', () => {
  it('identifies slow queries and connection pool pressure', () => {
    const analyzer = new DbAnalyzer();
    const result = analyzer.analyze([
      { name: 'db.query_p95_ms', value: 450, timestamp: Date.now(), labels: {}, family: 'Database' },
      { name: 'db.connection_pool_usage', value: 0.9, timestamp: Date.now(), labels: {}, family: 'Database' },
      { name: 'db.index_usage', value: 0.5, timestamp: Date.now(), labels: {}, family: 'Database' },
    ]);

    expect(result.anomalies).toContain('slowQuery');
    expect(result.anomalies).toContain('connectionPool');
  });
});
