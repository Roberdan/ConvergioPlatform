import type { Metric, OptimizationOpportunity } from '../core/types/index.js';
import { AnomalyDetector, type DetectedAnomaly } from './anomaly-detector.js';
import { PatternRecognizer } from './pattern-recognizer.js';

export interface AnalysisOutcome {
  anomalies: DetectedAnomaly[];
  opportunities: OptimizationOpportunity[];
}

export class AnalysisEngine {
  private readonly anomalyDetector = new AnomalyDetector();
  private readonly patternRecognizer = new PatternRecognizer();

  analyze(metrics: Metric[]): AnalysisOutcome {
    const byName = new Map<string, number[]>();
    for (const metric of metrics) {
      const bucket = byName.get(metric.name);
      if (bucket) {
        bucket.push(metric.value);
      } else {
        byName.set(metric.name, [metric.value]);
      }
    }

    const anomalies: DetectedAnomaly[] = [];
    const opportunities: OptimizationOpportunity[] = [];

    for (const [name, values] of byName.entries()) {
      if (values.length < 2) {
        continue;
      }

      const current = values[values.length - 1] ?? 0;
      const baseline = average(values.slice(0, -1));
      const anomaly = this.anomalyDetector.detect(name, current, baseline, 0.25);
      if (anomaly) {
        anomalies.push(anomaly);
      }

      const pattern = this.patternRecognizer.recognize(name, values);
      if (pattern && pattern.trend === 'up' && anomaly) {
        opportunities.push({
          title: `Investigate ${name}`,
          description: `Detected upward drift (${(pattern.drift * 100).toFixed(1)}%) with confidence ${(anomaly.confidence * 100).toFixed(0)}%`,
          estimatedGain: '-10% regression risk',
          domain: 'analysis',
          suggestedBlastRadius: 'SingleRepo',
        });
      }
    }

    return { anomalies, opportunities };
  }
}

function average(values: number[]): number {
  if (values.length === 0) {
    return 0;
  }
  const total = values.reduce((sum, value) => sum + value, 0);
  return total / values.length;
}
