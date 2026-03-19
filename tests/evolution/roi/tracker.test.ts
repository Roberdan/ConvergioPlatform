/**
 * Tests for RoiTracker — ROI computation from evolution experiment data.
 */

import { describe, it, expect } from 'vitest';
import { RoiTracker } from '../../../evolution/roi/tracker.js';

describe('RoiTracker', () => {
  it('returns empty summary when DB is not present', () => {
    const tracker = new RoiTracker({ dbPath: '/nonexistent/db.sqlite' });
    const summary = tracker.computeWeekly();
    expect(summary.proposalsGenerated).toBe(0);
    expect(summary.experimentsRun).toBe(0);
    expect(summary.rollbacks).toBe(0);
    expect(summary.improvementGains).toBe(0);
    expect(summary.systemCost).toBe(0);
    expect(summary.netROI).toBe(0);
    expect(summary.estimatedSavingsUsd).toBe(0);
  });

  it('period is a non-empty string', () => {
    const tracker = new RoiTracker({ dbPath: '/nonexistent/db.sqlite' });
    const summary = tracker.computeWeekly();
    expect(summary.period).toMatch(/^\d{4}-W\d{2}$/);
  });

  it('netROI equals improvementGains minus systemCost', () => {
    const tracker = new RoiTracker({ dbPath: '/nonexistent/db.sqlite' });
    const summary = tracker.computeWeekly();
    expect(summary.netROI).toBeCloseTo(summary.improvementGains - summary.systemCost);
  });

  it('estimatedSavingsUsd is non-negative', () => {
    const tracker = new RoiTracker({ dbPath: '/nonexistent/db.sqlite' });
    const summary = tracker.computeWeekly();
    expect(summary.estimatedSavingsUsd).toBeGreaterThanOrEqual(0);
  });
});
