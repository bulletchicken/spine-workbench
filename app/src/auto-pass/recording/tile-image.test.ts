import { apply } from '@/film-viewer/film-space';
import type { PreviewTile } from '@/shapes';
import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import { tileImage } from './tile-image';

function tile(fields: Partial<PreviewTile> & Pick<PreviewTile, 'width' | 'height' | 'data'>): PreviewTile {
  return { origin: [0, 0], stepAcross: [1, 0], stepDown: [0, 1], encoding: 'unit', ...fields };
}

describe('a preview tile as a bitmap', () => {
  it('turns each byte into an opaque grey pixel, row by row', () => {
    const image = tileImage(tile({ width: 2, height: 2, data: new Uint8Array([0, 128, 200, 255]) }));
    expect(image).not.toBeNull();
    expect(Array.from(image?.rgba ?? [])).toEqual([0, 0, 0, 255, 128, 128, 128, 255, 200, 200, 200, 255, 255, 255, 255, 255]);
  });

  it('places a shrunk tile so each pixel centre lands on its cell centre', () => {
    // Cells 4 film pixels apart from (10, 20): the pixel square (0..1, 0..1)
    // covers film (8..12, 18..22), whose centre is (10, 20).
    const image = tileImage(tile({ width: 2, height: 2, origin: [10, 20], stepAcross: [4, 0], stepDown: [0, 4], data: new Uint8Array(4) }));
    expect(image?.toFilm).toEqual([4, 0, 0, 4, 8, 18]);
    const toFilm = image?.toFilm ?? [1, 0, 0, 1, 0, 0];
    expect(apply(toFilm, [0.5, 0.5])).toEqual([10, 20]);
    expect(apply(toFilm, [1.5, 0.5])).toEqual([14, 20]);
    expect(apply(toFilm, [0.5, 1.5])).toEqual([10, 24]);
    expect(apply(toFilm, [0, 0])).toEqual([8, 18]);
  });

  it('places a tile computed on a turned copy of the film', () => {
    // Across runs down the film and down runs to the left: cell (i, j) sits
    // at (50 − 3j, 5 + 3i).
    const image = tileImage(tile({ width: 3, height: 2, origin: [50, 5], stepAcross: [0, 3], stepDown: [-3, 0], data: new Uint8Array(6) }));
    const toFilm = image?.toFilm ?? [1, 0, 0, 1, 0, 0];
    expect(apply(toFilm, [0.5, 0.5])).toEqual([50, 5]);
    expect(apply(toFilm, [2.5, 0.5])).toEqual([50, 11]);
    expect(apply(toFilm, [0.5, 1.5])).toEqual([47, 5]);
    expect(apply(toFilm, [2.5, 1.5])).toEqual([47, 11]);
  });

  it('lands every cell centre where the tile says, for any steps', () => {
    const vector = fc.tuple(fc.double({ min: -50, max: 50, noNaN: true }), fc.double({ min: -50, max: 50, noNaN: true }));
    fc.assert(
      fc.property(vector, vector, vector, fc.integer({ min: 0, max: 30 }), fc.integer({ min: 0, max: 30 }), (origin, across, down, i, j) => {
        const image = tileImage(tile({ width: 31, height: 31, origin, stepAcross: across, stepDown: down, data: new Uint8Array(31 * 31) }));
        const [x, y] = apply(image?.toFilm ?? [0, 0, 0, 0, Number.NaN, Number.NaN], [i + 0.5, j + 0.5]);
        expect(x).toBeCloseTo(origin[0] + i * across[0] + j * down[0], 9);
        expect(y).toBeCloseTo(origin[1] + i * across[1] + j * down[1], 9);
      }),
    );
  });

  it('shows the ridge value of a signed-pair tile, the first byte of each pair', () => {
    const image = tileImage(tile({ width: 2, height: 1, encoding: 'signed-pair', data: new Uint8Array([10, 200, 30, 40]) }));
    expect(Array.from(image?.rgba ?? [])).toEqual([10, 10, 10, 255, 30, 30, 30, 255]);
  });

  it('refuses a tile whose bytes do not fill its cells', () => {
    expect(tileImage(tile({ width: 2, height: 2, data: new Uint8Array(3) }))).toBeNull();
    expect(tileImage(tile({ width: 2, height: 2, data: new Uint8Array(5) }))).toBeNull();
    expect(tileImage(tile({ width: 2, height: 1, encoding: 'signed-pair', data: new Uint8Array(2) }))).toBeNull();
  });
});
