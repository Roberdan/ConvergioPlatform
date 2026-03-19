import type { Metric } from '../core/types/index.js';
import {
  LatencyEvaluator,
  MeshEvaluator,
  AgentCostEvaluator,
  WorkloadEvaluator,
  BundleEvaluator,
} from '../core/evaluators/index.js';

export class RuntimeAnalyzer {
  private readonly evaluator = new LatencyEvaluator();
  analyze(metrics: Metric[]) {
    return this.evaluator.evaluate(metrics, []);
  }
}

export class MeshAnalyzer {
  private readonly evaluator = new MeshEvaluator();
  analyze(metrics: Metric[]) {
    return this.evaluator.evaluate(metrics, []);
  }
}

export class AgentAnalyzer {
  private readonly evaluator = new AgentCostEvaluator();
  analyze(metrics: Metric[]) {
    return this.evaluator.evaluate(metrics, []);
  }
}

export class CostAnalyzer {
  private readonly evaluator = new AgentCostEvaluator();
  analyze(metrics: Metric[]) {
    return this.evaluator.evaluate(metrics, []);
  }
}

export class DXAnalyzer {
  private readonly evaluator = new BundleEvaluator();
  analyze(metrics: Metric[]) {
    return this.evaluator.evaluate(metrics, []);
  }
}

export function createDomainAnalyzers() {
  return {
    runtime: new RuntimeAnalyzer(),
    mesh: new MeshAnalyzer(),
    agent: new AgentAnalyzer(),
    cost: new CostAnalyzer(),
    dx: new DXAnalyzer(),
  };
}
