import { describe, expect, it } from 'vitest';
import type { View } from '../looking/view';
import { inTurn, settle, stretch, viewBetween } from './easing';

describe('helpers for animating the 3D view', () => {
  it('settle eases out and comes to rest', () => {
    expect(settle(0)).toBe(0);
    expect(settle(1)).toBe(1);
    expect(settle(0.25)).toBeGreaterThan(0.6);
    let last = 0;
    for (let t = 0.05; t <= 1; t += 0.05) {
      expect(settle(t)).toBeGreaterThanOrEqual(last);
      last = settle(t);
    }
  });

  it('a stretch of a motion runs from 0 to 1 between its two shares', () => {
    expect(stretch(0.1, 0.2, 0.6)).toBe(0);
    expect(stretch(0.4, 0.2, 0.6)).toBeCloseTo(0.5, 12);
    expect(stretch(0.9, 0.2, 0.6)).toBe(1);
  });

  it('items take their turn: each starts after the one before, the first at once and the last ending with the whole', () => {
    const count = 17;
    expect(inTurn(0, 0, count)).toBe(0);
    expect(inTurn(1e-6, 0, count)).toBeGreaterThan(0);
    expect(inTurn(1, count - 1, count)).toBe(1);
    expect(inTurn(0.999, count - 1, count)).toBeLessThan(1);
    for (let i = 1; i < count; i += 1) expect(inTurn(0.5, i, count)).toBeLessThanOrEqual(inTurn(0.5, i - 1, count));
    // About three are moving at once.
    const moving = Array.from({ length: count }, (_, i) => inTurn(0.5, i, count)).filter((own) => own > 0 && own < 1).length;
    expect(moving).toBeGreaterThanOrEqual(2);
    expect(moving).toBeLessThanOrEqual(3);
    expect(inTurn(0.4, 0, 1)).toBe(0.4);
  });

  it('a view between two views turns the short way round', () => {
    const from: View = { azimuth: 350, elevation: 0, perspective: 0, pxPerMm: [1, 1], aim: [0, 0, 0], aimAt: [0, 0] };
    const to: View = { azimuth: 10, elevation: 20, perspective: 1, pxPerMm: [2, 3], aim: [10, 20, 30], aimAt: [100, 50] };
    expect(viewBetween(from, to, 0.5)).toEqual({ azimuth: 360, elevation: 10, perspective: 0.5, pxPerMm: [1.5, 2], aim: [5, 10, 15], aimAt: [50, 25] });
    expect(viewBetween(from, to, 0)).toEqual(from);
  });
});
