import { describe, it, expect } from 'vitest';
import { AnomalyDetector } from './anomaly-detector.js';

describe('AnomalyDetector', () => {
  const detector = new AnomalyDetector();

  describe('zero baseline', () => {
    it('returns null when both current and baseline are zero', () => {
      const result = detector.detect('cpu', 0, 0, 0.1);
      expect(result).toBeNull();
    });

    it('returns a medium anomaly when baseline is zero and current is positive', () => {
      const result = detector.detect('cpu', 42, 0, 0.1);
      expect(result).not.toBeNull();
      expect(result?.severity).toBe('medium');
      expect(result?.confidence).toBe(1);
      expect(result?.metric).toBe('cpu');
      expect(result?.detail).toContain('new metric appeared');
    });

    it('returns null when baseline is negative (treated as zero-or-less)', () => {
      // negative baseline also guard-returns null for zero current
      const result = detector.detect('cpu', 0, -5, 0.1);
      expect(result).toBeNull();
    });

    it('returns anomaly when baseline is negative and current is positive', () => {
      const result = detector.detect('cpu', 10, -5, 0.1);
      expect(result).not.toBeNull();
      expect(result?.severity).toBe('medium');
    });
  });

  describe('normal operation', () => {
    it('returns null when change is within threshold', () => {
      const result = detector.detect('latency', 100, 100, 0.1);
      expect(result).toBeNull();
    });

    it('returns low severity for small deviation', () => {
      // ratio = 0.15, above 0.1 threshold, below 0.25
      const result = detector.detect('latency', 115, 100, 0.1);
      expect(result?.severity).toBe('low');
    });

    it('returns medium severity for moderate deviation', () => {
      // ratio = 0.30
      const result = detector.detect('latency', 130, 100, 0.1);
      expect(result?.severity).toBe('medium');
    });

    it('returns high severity for large deviation', () => {
      // ratio = 0.60
      const result = detector.detect('latency', 160, 100, 0.1);
      expect(result?.severity).toBe('high');
    });

    it('includes current/baseline/ratio in detail string', () => {
      const result = detector.detect('latency', 150, 100, 0.1);
      expect(result?.detail).toMatch(/current=150\.00/);
      expect(result?.detail).toMatch(/baseline=100\.00/);
      expect(result?.detail).toMatch(/ratio=0\.500/);
    });
  });
});
