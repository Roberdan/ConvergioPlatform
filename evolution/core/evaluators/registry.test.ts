import { describe, expect, it } from 'vitest';
import { EvaluatorRegistry } from './registry.js';

describe('EvaluatorRegistry', () => {
  it('contains all five runtime evaluators', () => {
    const registry = new EvaluatorRegistry();
    const domains = registry.getAll().map((evaluator) => evaluator.domain).sort();
    expect(domains).toEqual(['bundle', 'database', 'latency', 'mesh_topology', 'workload']);
  });

  it('filters evaluators by metric family', () => {
    const registry = new EvaluatorRegistry();
    const filtered = registry.getForFamilies(['Database']);
    expect(filtered.map((evaluator) => evaluator.domain)).toContain('database');
  });
});
