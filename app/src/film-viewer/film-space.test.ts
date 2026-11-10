import type { Px, QuarterTurns } from '@/shapes';
import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { apply, filmSpace, uprightMatrix, uprightSize, type FilmPlacement } from './film-space';

const placements = fc.record({
  size: fc.record({ width: fc.integer({ min: 1, max: 6000 }), height: fc.integer({ min: 1, max: 6000 }) }),
  view: fc.record({
    quarterTurns: fc.constantFrom<QuarterTurns>(0, 1, 2, 3),
    mirrored: fc.boolean(),
  }),
  zoom: fc.double({ min: 0.01, max: 64, noNaN: true }),
  pan: fc.tuple(fc.double({ min: -1e4, max: 1e4, noNaN: true }), fc.double({ min: -1e4, max: 1e4, noNaN: true })),
}) satisfies fc.Arbitrary<FilmPlacement>;

function close(a: readonly number[], b: readonly number[], tolerance = 1e-6) {
  expect(a).toHaveLength(b.length);
  a.forEach((value, i) => {
    expect(Math.abs(value - (b[i] ?? Number.NaN))).toBeLessThan(tolerance * Math.max(1, Math.abs(value)));
  });
}

describe('film space', () => {
  it('turns clockwise: the top-left pixel goes to the top right', () => {
    const size = { width: 4, height: 3 };
    expect(apply(uprightMatrix(size, { quarterTurns: 1, mirrored: false }), [0, 0])).toEqual([2, 0]);
    expect(apply(uprightMatrix(size, { quarterTurns: 2, mirrored: false }), [0, 0])).toEqual([3, 2]);
    expect(apply(uprightMatrix(size, { quarterTurns: 3, mirrored: false }), [0, 0])).toEqual([0, 3]);
    expect(apply(uprightMatrix(size, { quarterTurns: 0, mirrored: true }), [0, 0])).toEqual([3, 0]);
  });

  it('puts the centre of the top-left pixel half a pixel in from the corner', () => {
    const space = filmSpace({ size: { width: 10, height: 20 }, view: { quarterTurns: 0, mirrored: false }, zoom: 2, pan: [100, 50] });
    expect(space.toScreen([0, 0])).toEqual([101, 51]);
    expect(space.toScreen([-0.5, -0.5])).toEqual([100, 50]);
  });

  it('round-trips every point under every turn and mirror', () => {
    fc.assert(
      fc.property(placements, fc.double({ min: 0, max: 1, noNaN: true }), fc.double({ min: 0, max: 1, noNaN: true }), (placement, u, v) => {
        const space = filmSpace(placement);
        const p: [number, number] = [u * placement.size.width - 0.5, v * placement.size.height - 0.5];
        close(space.toFilm(space.toScreen(p)), p, 1e-6);
      }),
    );
  });

  it('maps the film exactly onto its upright rectangle on screen', () => {
    fc.assert(
      fc.property(placements, (placement) => {
        const space = filmSpace(placement);
        const { width, height } = placement.size;
        const filmCorners: Px[] = [[-0.5, -0.5], [width - 0.5, -0.5], [-0.5, height - 0.5], [width - 0.5, height - 0.5]];
        const corners = filmCorners
          .map((corner) => space.toScreen(corner))
          .map(([x, y]) => [x - placement.pan[0], y - placement.pan[1]])
          .sort((a, b) => Math.round(((a[1] ?? 0) - (b[1] ?? 0)) * 1e3) || (a[0] ?? 0) - (b[0] ?? 0));
        const upright = uprightSize(placement.size, placement.view.quarterTurns);
        const z = placement.zoom;
        const expected = [
          [0, 0],
          [upright.width * z, 0],
          [0, upright.height * z],
          [upright.width * z, upright.height * z],
        ];
        corners.forEach((corner, i) => {
          close(corner, expected[i] ?? [], 1e-6);
        });
      }),
    );
  });

  it('keeps a length on screen the same size at any zoom', () => {
    const space = filmSpace({ size: { width: 10, height: 10 }, view: { quarterTurns: 1, mirrored: true }, zoom: 4, pan: [0, 0] });
    expect(space.screenToFilmLength(8)).toBe(2);
  });
});
