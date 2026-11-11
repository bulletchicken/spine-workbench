import type { Px } from '@/shapes';
import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { gapInSpan } from './gap-in-span';
import type { FilmStrip, StripShape } from './pass-recording';
import { nearestRow } from './strip-position';

/** A straight strip down x = 100: row r is centred at (100, 10 + 2r). */
function straight(rows: number): StripShape {
  return {
    centres: Array.from({ length: rows }, (_, r): Px => [100, 10 + 2 * r]),
    normals: Array.from({ length: rows }, (): Px => [1, 0]),
    columnPx: 1,
    columns: 41,
  };
}

function strip(shape: StripShape, highPass: number[]): Pick<FilmStrip, 'shape' | 'barcode'> {
  return { shape, barcode: { values: Float32Array.from(highPass), highPass: Float32Array.from(highPass) } };
}

/** Bone everywhere, with a dip of `depth` below zero centred at `row`. */
function dip(rows: number, row: number, depth = 2, width = 3): number[] {
  return Array.from({ length: rows }, (_, r) => 1 - (1 + depth) * Math.exp(-(((r - row) / width) ** 2)));
}

const at = (row: number): Px => [100, 10 + 2 * row];

describe('the gap inside a span', () => {
  it('finds a made-up gap and gives back its row’s centre on the film', () => {
    // Rows 60 to 120: the search keeps to rows 75 to 105, and the gap is
    // at row 87, centred at (100, 10 + 2·87) = (100, 184).
    expect(gapInSpan(strip(straight(200), dip(200, 87)), at(60), at(120))).toEqual([100, 184]);
  });

  it('does not care which end comes first or how far across the spine the ends are', () => {
    const s = strip(straight(200), dip(200, 87));
    expect(gapInSpan(s, at(120), at(60))).toEqual([100, 184]);
    expect(gapInSpan(s, [130, 130], [72, 250])).toEqual([100, 184]);
  });

  it('keeps a quarter of the span clear of each end', () => {
    // A deeper dip at row 63 lies in the first quarter of rows 60 to 120.
    const profile = dip(200, 87, 1).map((v, r) => Math.min(v, 1 - 4 * Math.exp(-(((r - 63) / 2) ** 2))));
    expect(gapInSpan(strip(straight(200), profile), at(60), at(120))).toEqual(at(87));
    // With the gap itself inside the last quarter, there is none to give.
    expect(gapInSpan(strip(straight(200), dip(200, 110)), at(60), at(120))).toBeNull();
  });

  it('takes the deeper of two gaps', () => {
    const profile = dip(200, 80, 1).map((v, r) => Math.min(v, 1 - 3 * Math.exp(-(((r - 98) / 2) ** 2))));
    expect(gapInSpan(strip(straight(200), profile), at(60), at(120))).toEqual(at(98));
  });

  it('finds none where the profile never drops below zero', () => {
    expect(gapInSpan(strip(straight(200), dip(200, 87, -0.5)), at(60), at(120))).toBeNull();
  });

  it('finds none on a slope that is only lowest at the edge of the search', () => {
    const slope = Array.from({ length: 200 }, (_, r) => -r / 100);
    expect(gapInSpan(strip(straight(200), slope), at(60), at(120))).toBeNull();
  });

  it('passes over rows with no value', () => {
    const profile = dip(200, 87);
    profile[87] = Number.NaN;
    const found = gapInSpan(strip(straight(200), profile), at(60), at(120));
    expect(found === null ? null : nearestRow(straight(200), found)).toBe(86);
  });

  it('finds a made-up gap to within one row on a curved strip, wherever it lies in the middle of the span', () => {
    const rows = 400;
    const curved: StripShape = {
      centres: Array.from({ length: rows }, (_, r): Px => [200 + 40 * Math.sin(r / 50), 30 + 2 * r]),
      normals: Array.from({ length: rows }, (_, r): Px => {
        const slope = (40 / 50) * Math.cos(r / 50) / 2;
        const length = Math.hypot(1, slope);
        return [1 / length, -slope / length];
      }),
      columnPx: 1,
      columns: 41,
    };
    const spans = fc
      .record({ from: fc.integer({ min: 2, max: 300 }), length: fc.integer({ min: 24, max: 95 }) })
      .chain(({ from, length }) =>
        fc.record({
          from: fc.constant(from),
          to: fc.constant(from + length),
          gap: fc.double({ min: from + 0.3 * length, max: from + 0.7 * length, noNaN: true }),
          width: fc.double({ min: 1.5, max: 4, noNaN: true }),
          aside: fc.double({ min: -6, max: 6, noNaN: true }),
        }),
      );
    fc.assert(
      fc.property(spans, ({ from, to, gap, width, aside }) => {
        const point = (row: number): Px => {
          const [x, y] = curved.centres[row] ?? [0, 0];
          const [nx, ny] = curved.normals[row] ?? [1, 0];
          return [x + aside * nx, y + aside * ny];
        };
        const found = gapInSpan(strip(curved, dip(rows, gap, 2, width)), point(from), point(to));
        expect(found).not.toBeNull();
        const row = found === null ? Number.NaN : nearestRow(curved, found);
        expect(Math.abs((row ?? Number.NaN) - gap)).toBeLessThanOrEqual(1);
      }),
    );
  });
});
