import type { MetricFamily } from '../types/index.js';
import type { Evaluator } from './evaluator.js';
import {
  BundleEvaluator,
  DbEvaluator,
  LatencyEvaluator,
  MeshTopologyEvaluator,
  WorkloadEvaluator,
} from './domains/index.js';

export class EvaluatorRegistry {
  private readonly evaluators: Evaluator[];

  constructor(initial: Evaluator[] = [
    new LatencyEvaluator(),
    new BundleEvaluator(),
    new MeshTopologyEvaluator(),
    new DbEvaluator(),
    new WorkloadEvaluator(),
  ]) {
    this.evaluators = [...initial];
  }

  getAll(): Evaluator[] {
    return [...this.evaluators];
  }

  getForFamilies(families: MetricFamily[]): Evaluator[] {
    const familySet = new Set(families);
    return this.evaluators.filter((evaluator) =>
      evaluator.metricFamilies.some((family) => familySet.has(family)),
    );
  }

  register(evaluator: Evaluator): void {
    this.evaluators.push(evaluator);
  }
}
