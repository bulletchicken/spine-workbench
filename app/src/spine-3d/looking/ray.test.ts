import { describe, expect, it } from 'vitest';
import { type Vec3, cross, length, sub } from '../placing/vectors';
import { toCanvas } from './lens';
import { rayThrough } from './ray';
import type { View } from './view';

const canvas = { width: 800, height: 600 };
const POINTS: Vec3[] = [
  [10, 200, -20],
  [60, 420, 35],
  [-80, 15, -90],
];

describe('the line behind a place on the canvas', () => {
  it('passes through every point that is drawn at that place, with or without perspective', () => {
    for (const perspective of [0, 0.4, 1]) {
      const view: View = { azimuth: 34, elevation: 14, perspective, pxPerMm: [1.5, 1.5], aim: [10, 200, -20], aimAt: [380, 310] };
      for (const point of POINTS) {
        const drawn = toCanvas(view, canvas, point);
        const ray = rayThrough(view, canvas, drawn.x, drawn.y, 700);
        // The point's distance from the line.
        expect(length(cross(sub(point, ray.from), ray.toward))).toBeLessThan(1e-6);
        expect(length(ray.toward)).toBeCloseTo(1, 12);
      }
    }
  });
});
