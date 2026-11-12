import { describe, expect, it } from 'vitest';
import type { Vec3 } from '../placing/vectors';
import { LENS_DEGREES, eyeDistance, lensMatrices, toCanvas } from './lens';
import type { View } from './view';

const canvas = { width: 800, height: 600 };

const view = (over: Partial<View> = {}): View => ({
  azimuth: 34,
  elevation: 14,
  perspective: 1,
  pxPerMm: [1.5, 1.5],
  aim: [10, 200, -20],
  aimAt: [400, 300],
  ...over,
});

const POINTS: Vec3[] = [
  [10, 200, -20],
  [60, 420, 35],
  [-80, 15, -90],
  [0, 0, 0],
];

/** A scene point through the two matrices, as a GPU would take it: to CSS pixels and a depth from −1 (near) to 1 (far). */
function throughMatrices(v: View, point: Vec3) {
  const { world, projection } = lensMatrices(v, canvas, 700);
  const at = (m: number[], row: number, column: number) => m[column * 4 + row] ?? 0;
  // The camera's matrix is a rotation and a place, so its inverse is the transpose and the place undone.
  const from = [point[0] - at(world, 0, 3), point[1] - at(world, 1, 3), point[2] - at(world, 2, 3)] as const;
  const eye = [0, 1, 2].map((axis) => at(world, 0, axis) * from[0] + at(world, 1, axis) * from[1] + at(world, 2, axis) * from[2]);
  const clip = [0, 1, 2, 3].map((row) => at(projection, row, 0) * (eye[0] ?? 0) + at(projection, row, 1) * (eye[1] ?? 0) + at(projection, row, 2) * (eye[2] ?? 0) + at(projection, row, 3));
  const w = clip[3] ?? 1;
  return { x: (((clip[0] ?? 0) / w + 1) * canvas.width) / 2, y: ((1 - (clip[1] ?? 0) / w) * canvas.height) / 2, depth: (clip[2] ?? 0) / w };
}

describe('the lens', () => {
  it('draws the aim where it is told to, at every perspective', () => {
    for (const perspective of [0, 0.01, 0.5, 1]) {
      const drawn = toCanvas(view({ perspective, aimAt: [123, 456] }), canvas, [10, 200, -20]);
      expect([drawn.x, drawn.y, drawn.nearer]).toEqual([123, 456, 0]);
    }
  });

  it('with no perspective draws a millimetre the same size at any depth', () => {
    const flat = view({ azimuth: 0, elevation: 0, perspective: 0 });
    const near = toCanvas(flat, canvas, [40, 200, 300]);
    const far = toCanvas(flat, canvas, [40, 200, -300]);
    expect(near.x).toBe(far.x);
    expect(near.x).toBe(400 + 1.5 * 30);
    expect(near.nearer - far.nearer).toBe(600);
  });

  it('with the lens draws what is nearer larger, from an eye that sees the picture in 18 degrees', () => {
    const lens = view({ azimuth: 0, elevation: 0 });
    const distance = eyeDistance(lens, canvas);
    // Half the picture is 200 mm tall at the aim; tan 9° = 0.158384…
    expect(distance).toBeCloseTo(200 / Math.tan((LENS_DEGREES / 2) * (Math.PI / 180)), 9);
    const near = toCanvas(lens, canvas, [40, 200, 80]);
    const far = toCanvas(lens, canvas, [40, 200, -120]);
    expect(near.x - 400).toBeGreaterThan(far.x - 400);
    expect(near.x - 400).toBeCloseTo((45 * (distance ?? 0)) / ((distance ?? 0) - 100), 9);
    expect(eyeDistance(view({ perspective: 0 }), canvas)).toBeNull();
  });

  it('gives the GPU two matrices that draw every point where the plain rule does', () => {
    for (const v of [view(), view({ perspective: 0 }), view({ perspective: 0.3, azimuth: -90, elevation: 0 }), view({ azimuth: 0, elevation: 90, perspective: 0, pxPerMm: [2, 1.6], aimAt: [250, 410] })]) {
      for (const point of POINTS) {
        const plain = toCanvas(v, canvas, point);
        const drawn = throughMatrices(v, point);
        expect(Math.hypot(drawn.x - plain.x, drawn.y - plain.y)).toBeLessThan(1e-9);
        expect(Math.abs(drawn.depth)).toBeLessThan(1);
      }
    }
  });

  it('keeps what is nearer in front: depth falls as a point comes toward the viewer', () => {
    for (const perspective of [0, 1]) {
      const v = view({ azimuth: 0, elevation: 0, perspective });
      expect(throughMatrices(v, [10, 200, 100]).depth).toBeLessThan(throughMatrices(v, [10, 200, -100]).depth);
      expect(lensMatrices(v, canvas, 700).flat).toBe(perspective === 0);
    }
  });
});
