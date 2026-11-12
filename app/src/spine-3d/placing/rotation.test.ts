import { describe, expect, it } from 'vitest';
import { matrixFromRotationVector } from './rotation';
import { type Mat3, apply, determinant, times, transpose } from './vectors';

const close = (a: Mat3, b: Mat3, within = 1e-15) => {
  for (const i of [0, 1, 2] as const) for (const j of [0, 1, 2] as const) expect(Math.abs(a[i][j] - b[i][j])).toBeLessThanOrEqual(within);
};

describe('a rotation vector as a matrix', () => {
  it('is no rotation for the zero vector', () => {
    close(matrixFromRotationVector([0, 0, 0]), [[1, 0, 0], [0, 1, 0], [0, 0, 1]], 0);
  });

  it('is the rotation about an axis for a vector along it, with the engine\'s signs', () => {
    const angle = 0.7;
    const [s, c] = [Math.sin(angle), Math.cos(angle)];
    // rotX(+θ) lowers the right side, rotY(+φ) lowers the front, rotZ(+ψ) turns the front to the left.
    close(matrixFromRotationVector([angle, 0, 0]), [[1, 0, 0], [0, c, -s], [0, s, c]]);
    close(matrixFromRotationVector([0, angle, 0]), [[c, 0, s], [0, 1, 0], [-s, 0, c]]);
    close(matrixFromRotationVector([0, 0, angle]), [[c, -s, 0], [s, c, 0], [0, 0, 1]]);
  });

  it('turns forward to the left for a quarter turn about up', () => {
    const r = matrixFromRotationVector([0, 0, Math.PI / 2]);
    const [x, y, z] = apply(r, [1, 0, 0]);
    expect(Math.hypot(x, y - 1, z)).toBeLessThan(1e-15);
  });

  it('is always a rotation: its transpose undoes it and it does not mirror', () => {
    for (const r of [[0.3, -0.7, 1.2], [1e-9, -2e-9, 5e-10], [-2.1, 0.4, 0.9]] as const) {
      const m = matrixFromRotationVector(r);
      close(times(m, transpose(m)), [[1, 0, 0], [0, 1, 0], [0, 0, 1]], 1e-14);
      expect(determinant(m)).toBeCloseTo(1, 12);
    }
  });

  it('keeps its precision for a tiny angle', () => {
    // For θ = 1e-9 the matrix differs from I + K by θ²/2, about 5e-19.
    const m = matrixFromRotationVector([1e-9, -2e-9, 5e-10]);
    close(m, [[1, -5e-10, -2e-9], [5e-10, 1, -1e-9], [2e-9, 1e-9, 1]], 1e-17);
  });
});
