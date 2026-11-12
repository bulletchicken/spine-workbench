import { describe, expect, it } from 'vitest';
import type { Stage } from '@/shapes';
import { searchShare } from './search-progress';

describe('search progress', () => {
  it('gives the readings nine tenths of a Front-only search', () => {
    expect(searchShare({ stage: 'evidence', fraction: 0.5 }, true)).toBe(0.45);
    expect(searchShare({ stage: 'evidence', fraction: 1 }, true)).toBe(0.9);
    expect(searchShare({ stage: 'chain', fraction: 0 }, true)).toBe(0.93);
    expect(searchShare({ stage: 'naming', fraction: 1 }, true)).toBe(0.96);
    expect(searchShare({ stage: 'confidence', fraction: 1 }, true)).toBe(1);
  });
  it('joins the six stages without gaps and holds fractions to their bounds', () => {
    const stages: Stage[] = ['evidence', 'centre-line', 'strip', 'chain', 'naming', 'confidence'];
    const ends = [0.30, 0.45, 0.55, 0.85, 0.90, 1];
    stages.forEach((stage, i) => {
      expect(searchShare({ stage, fraction: 0 }, false)).toBeCloseTo(ends[i - 1] ?? 0);
      expect(searchShare({ stage, fraction: 2 }, false)).toBeCloseTo(ends[i] ?? 1);
      expect(searchShare({ stage, fraction: NaN }, false)).toBeGreaterThanOrEqual(0);
      expect(searchShare({ stage, fraction: -1 }, true)).toBeLessThanOrEqual(1);
    });
  });
});
