export type Trend = 'up' | 'down' | 'flat';

export interface PatternSignal {
  metric: string;
  trend: Trend;
  drift: number;
}

export class PatternRecognizer {
  recognize(metric: string, values: number[]): PatternSignal | null {
    if (values.length < 3) {
      return null;
    }

    const first = values[0] ?? 0;
    const last = values[values.length - 1] ?? 0;
    const drift = first === 0 ? 0 : (last - first) / first;
    const trend: Trend = Math.abs(drift) < 0.05 ? 'flat' : drift > 0 ? 'up' : 'down';

    return {
      metric,
      trend,
      drift,
    };
  }
}
