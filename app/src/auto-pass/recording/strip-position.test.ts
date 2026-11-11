import type { Px } from '@/shapes';
import { describe, expect, it } from 'vitest';
import type { StripShape } from './pass-recording';
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

describe('the strip row of a film point', () => {
  it('is the row whose centre is nearest', () => {
    expect(nearestRow(straight(10), [100, 14])).toBe(2);
    expect(nearestRow(straight(10), [140, 16.9])).toBe(3);
    expect(nearestRow(straight(10), [100, -50])).toBe(0);
    expect(nearestRow(straight(10), [100, 500])).toBe(9);
  });

  it('is none on a strip with no rows', () => {
    expect(nearestRow(straight(0), [0, 0])).toBeNull();
  });
});
