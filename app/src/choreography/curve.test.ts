// Known answers for a cubic-bezier come from its own definition: at
// parameter u the curve is at
//
//   x = 3(1-u)²u·x1 + 3(1-u)u²·x2 + u³      and the same for y.
//
// At u = 1/2 that is x = (3·x1 + 3·x2 + 1) / 8, so each of the design's
// curves has one point that can be worked out by hand:
//
//   --ease-out     (.2, .8, .2, 1)    x = 2.2/8 = .275     y = 6.4/8 = .8
//   --ease-in-out  (.65, 0, .35, 1)   x = 4/8   = .5       y = 4/8   = .5
//   --ease-lens    (.7, 0, .2, 1)     x = 3.7/8 = .4625    y = 4/8   = .5
//   --ease-snap    (.2, 1.4, .4, 1)   x = 2.8/8 = .35      y = 8.2/8 = 1.025
//
// The property tests compare against a slow search that walks the curve in
// small steps of u and knows nothing of how cubicBezier finds its answer.

import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { cubicBezier, linear } from './curve';

/** The curve's y where its x first reaches `time`, found by walking u in two hundred thousand steps. */
function byWalking(x1: number, y1: number, x2: number, y2: number, time: number): number {
  const at = (u: number, p1: number, p2: number) => 3 * (1 - u) * (1 - u) * u * p1 + 3 * (1 - u) * u * u * p2 + u * u * u;
  const steps = 200_000;
  for (let step = 0; step <= steps; step += 1) {
    const u = step / steps;
    if (at(u, x1, x2) >= time) return at(u, y1, y2);
  }
  return 1;
}

const unit = fc.double({ min: 0, max: 1, noNaN: true });

describe('cubicBezier', () => {
  it('passes through the hand-worked point of each of the design’s curves', () => {
    expect(cubicBezier(0.2, 0.8, 0.2, 1)(0.275)).toBeCloseTo(0.8, 12);
    expect(cubicBezier(0.65, 0, 0.35, 1)(0.5)).toBeCloseTo(0.5, 12);
    expect(cubicBezier(0.7, 0, 0.2, 1)(0.4625)).toBeCloseTo(0.5, 12);
    expect(cubicBezier(0.2, 1.4, 0.4, 1)(0.35)).toBeCloseTo(1.025, 12);
  });

  it('starts at 0 and ends at 1, and holds there outside its time', () => {
    const curve = cubicBezier(0.2, 1.4, 0.4, 1);
    expect(curve(0)).toBe(0);
    expect(curve(1)).toBe(1);
    expect(curve(-0.5)).toBe(0);
    expect(curve(1.5)).toBe(1);
  });

  it('is a straight line when the control points lie on the diagonal', () => {
    const straight = cubicBezier(0.25, 0.25, 0.75, 0.75);
    for (const time of [0.1, 0.3, 0.5, 0.77, 0.9]) expect(straight(time)).toBeCloseTo(time, 12);
  });

  it('overshoots when a control point is above 1, as --ease-snap does', () => {
    const snap = cubicBezier(0.2, 1.4, 0.4, 1);
    const highest = Math.max(...Array.from({ length: 99 }, (_, i) => snap((i + 1) / 100)));
    expect(highest).toBeGreaterThan(1);
    expect(highest).toBeLessThan(1.1);
  });

  it('refuses control points that would turn back in time', () => {
    expect(() => cubicBezier(-0.1, 0, 0.5, 1)).toThrow(RangeError);
    expect(() => cubicBezier(0.5, 0, 1.1, 1)).toThrow(RangeError);
  });

  it('refuses a control point that is not a number', () => {
    expect(() => cubicBezier(Number.NaN, 0, 0.5, 1)).toThrow(RangeError);
    expect(() => cubicBezier(0.5, 0, Number.NaN, 1)).toThrow(RangeError);
    expect(() => cubicBezier(0.5, Number.NaN, 0.5, 1)).toThrow(RangeError);
    expect(() => cubicBezier(0.5, 0, 0.5, Number.NaN)).toThrow(RangeError);
  });

  it('agrees with a slow walk along the curve', () => {
    fc.assert(
      fc.property(unit, fc.double({ min: -0.5, max: 1.5, noNaN: true }), unit, fc.double({ min: -0.5, max: 1.5, noNaN: true }), unit, (x1, y1, x2, y2, time) => {
        // The walk overshoots x by at most one step; the curve is no steeper than 3·2.5 there, plus slack.
        expect(Math.abs(cubicBezier(x1, y1, x2, y2)(time) - byWalking(x1, y1, x2, y2, time))).toBeLessThan(1e-3);
      }),
      { numRuns: 60 },
    );
  });

  it('never goes backwards when its control points stay between 0 and 1', () => {
    fc.assert(
      fc.property(unit, unit, unit, unit, unit, unit, (x1, y1, x2, y2, a, b) => {
        const curve = cubicBezier(x1, y1, x2, y2);
        const [earlier, later] = a <= b ? [a, b] : [b, a];
        expect(curve(later)).toBeGreaterThanOrEqual(curve(earlier) - 1e-9);
      }),
    );
  });
});

describe('linear', () => {
  it('answers the time itself, held between 0 and 1', () => {
    expect(linear(0.37)).toBe(0.37);
    expect(linear(-1)).toBe(0);
    expect(linear(2)).toBe(1);
  });
});
