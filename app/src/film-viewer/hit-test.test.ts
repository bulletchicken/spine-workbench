import type { Px } from '@/shapes';
import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { filmSpace } from './film-space';
import { nearestWithin, PIN_REACH, pointInQuad } from './hit-test';

const square: Px[] = [
  [10, 10],
  [20, 10],
  [20, 20],
  [10, 20],
];

describe('a point in a quad', () => {
  it('knows inside from outside', () => {
    expect(pointInQuad([15, 15], square)).toBe(true);
    expect(pointInQuad([25, 15], square)).toBe(false);
    expect(pointInQuad([15, 9.99], square)).toBe(false);
  });

  it('gives a shared edge to exactly one of two stacked outlines', () => {
    const below: Px[] = [
      [10, 20],
      [20, 20],
      [20, 30],
      [10, 30],
    ];
    for (const x of [10.5, 12, 15, 19.5]) {
      const claims = [pointInQuad([x, 20], square), pointInQuad([x, 20], below)].filter(Boolean);
      expect(claims).toHaveLength(1);
    }
  });

  it('gives a shared edge to exactly one of two outlines side by side: the one it is the left edge of', () => {
    const right: Px[] = [
      [20, 10],
      [30, 10],
      [30, 20],
      [20, 20],
    ];
    for (const y of [10.5, 12, 15, 19.5]) {
      expect(pointInQuad([20, y], square)).toBe(false);
      expect(pointInQuad([20, y], right)).toBe(true);
      expect(pointInQuad([10, y], square)).toBe(true);
    }
  });

  it('follows a tilted outline, corners in either order', () => {
    // A square turned 45° about (0, 0): its corners are 10 from the middle.
    const diamond: Px[] = [
      [0, -10],
      [10, 0],
      [0, 10],
      [-10, 0],
    ];
    expect(pointInQuad([4, 4], diamond)).toBe(true);
    expect(pointInQuad([6, 6], diamond)).toBe(false);
    expect(pointInQuad([4, 4], [...diamond].reverse())).toBe(true);
    expect(pointInQuad([-9, 0.5], diamond)).toBe(true);
    expect(pointInQuad([-9, 1.5], diamond)).toBe(false);
  });

  it('agrees with a slower test by triangles for any convex quad', () => {
    // A convex quad is two triangles; a point is inside a triangle when it
    // is on the same side of all three edges.
    const side = (a: Px, b: Px, p: Px) => (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
    const inTriangle = (a: Px, b: Px, c: Px, p: Px) => {
      const s = [side(a, b, p), side(b, c, p), side(c, a, p)];
      return s.every((v) => v > 1e-9) || s.every((v) => v < -1e-9);
    };
    const coordinate = fc.double({ min: -50, max: 50, noNaN: true });
    fc.assert(
      fc.property(
        fc.double({ min: 5, max: 40, noNaN: true }),
        fc.double({ min: 5, max: 40, noNaN: true }),
        fc.double({ min: 0, max: Math.PI, noNaN: true }),
        fc.double({ min: -0.4, max: 0.4, noNaN: true }),
        coordinate,
        coordinate,
        (w, h, angle, skew, px, py) => {
          const turn = ([x, y]: Px): Px => [x * Math.cos(angle) - y * Math.sin(angle), x * Math.sin(angle) + y * Math.cos(angle)];
          const quad: Px[] = (
            [
              [-w, -h],
              [w, -h * (1 + skew)],
              [w, h],
              [-w, h * (1 - skew)],
            ] as Px[]
          ).map(turn);
          const [a, b, c, d] = quad as [Px, Px, Px, Px];
          const p: Px = [px, py];
          const insideA = inTriangle(a, b, c, p);
          const insideB = inTriangle(a, c, d, p);
          const onDiagonal = Math.abs(side(a, c, p)) < 1e-6;
          fc.pre(!onDiagonal);
          const clearOfEdges = [side(a, b, p), side(b, c, p), side(c, d, p), side(d, a, p)].every((v) => Math.abs(v) > 1e-6);
          fc.pre(clearOfEdges);
          expect(pointInQuad(p, quad)).toBe(insideA || insideB);
        },
      ),
    );
  });
});

describe('the nearest point within reach', () => {
  const size = { width: 3000, height: 7000 };
  const corners: Px[] = [
    [1000, 2000],
    [1200, 2010],
    [1190, 2130],
    [1005, 2120],
  ];

  it('answers within twelve screen pixels at fit and at 400%, whatever the turn', () => {
    for (const zoom of [0.1, 0.4]) {
      for (const quarterTurns of [0, 1, 2, 3] as const) {
        const space = filmSpace({ size, view: { quarterTurns, mirrored: false }, zoom, pan: [17, -230] });
        const corner = corners[2] ?? [0, 0];
        // 11 and 13 screen pixels away, along a diagonal.
        const near = 11 / zoom / Math.SQRT2;
        const far = 13 / zoom / Math.SQRT2;
        expect(nearestWithin(corners, [corner[0] + near, corner[1] + near], PIN_REACH, space)).toBe(2);
        expect(nearestWithin(corners, [corner[0] + far, corner[1] + far], PIN_REACH, space)).toBeNull();
      }
    }
  });

  it('answers a point exactly twelve screen pixels away', () => {
    // At zoom 2, 6 film pixels across is 12 screen pixels, exactly in floating point.
    const space = filmSpace({ size, view: { quarterTurns: 0, mirrored: false }, zoom: 2, pan: [0, 0] });
    expect(nearestWithin(corners, [1006, 2000], PIN_REACH, space)).toBe(0);
    expect(nearestWithin(corners, [1006.01, 2000], PIN_REACH, space)).toBeNull();
  });

  it('picks the nearer of two points in reach', () => {
    const space = filmSpace({ size, view: { quarterTurns: 0, mirrored: false }, zoom: 0.05, pan: [0, 0] });
    // At 0.05 the corners are 10 and 6 screen pixels apart; the pointer between them goes to the nearer.
    expect(nearestWithin(corners, [1150, 2005], PIN_REACH, space)).toBe(1);
    expect(nearestWithin(corners, [1040, 2002], PIN_REACH, space)).toBe(0);
  });
});
