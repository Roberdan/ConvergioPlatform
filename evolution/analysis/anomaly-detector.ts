export type AnomalySeverity = 'low' | 'medium' | 'high';

export interface DetectedAnomaly {
  metric: string;
  severity: AnomalySeverity;
  detail: string;
  confidence: number;
}

export class AnomalyDetector {
  detect(metric: string, current: number, baseline: number, threshold: number): DetectedAnomaly | null {
    if (baseline <= 0) {
      return null;
    }

    const ratio = Math.abs(current - baseline) / baseline;
    if (ratio <= threshold) {
      return null;
    }

    const severity: AnomalySeverity = ratio > 0.5 ? 'high' : ratio > 0.25 ? 'medium' : 'low';
    const confidence = Math.min(1, Math.max(0, ratio));

    return {
      metric,
      severity,
      detail: `current=${current.toFixed(2)} baseline=${baseline.toFixed(2)} ratio=${ratio.toFixed(3)}`,
      confidence,
    };
  }
}
