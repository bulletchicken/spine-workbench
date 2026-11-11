import fc from 'fast-check';
import { describe, expect, it } from 'vitest';
import type { StripBlock } from './pass-recording';
import { stripImage } from './strip-image';

function block(firstRow: number, rows: number, columns: number, image: number[], valid: number[]): StripBlock {
  return { kind: 'strip-rows', film: 'front', stage: 'strip', firstRow, rows, columns, image: new Uint8Array(image), valid: new Uint8Array(valid) };
}

/** Packs one flag per cell into bytes, lowest bit first, the slow way. */
function packBits(flags: boolean[]): number[] {
  const bytes: number[] = [];
  flags.forEach((flag, cell) => {
    const byte = Math.floor(cell / 8);
    bytes[byte] = (bytes[byte] ?? 0) + (flag ? 2 ** (cell % 8) : 0);
  });
  return bytes;
}

describe('the strip bitmap', () => {
  it('joins blocks that arrive out of order, transparent where a cell was not usable', () => {
    // Rows 1 and 2 first: validity 0b101101 marks cells 0, 2, 3 and 5 usable.
    // Then row 0: 0b011 marks cells 0 and 1 usable.
    const image = stripImage([block(1, 2, 3, [1, 2, 3, 4, 5, 6], [0b101101]), block(0, 1, 3, [7, 8, 9], [0b011])], 3, 3);
    expect(image?.rows).toBe(3);
    expect(image?.columns).toBe(3);
    expect(Array.from(image?.rgba ?? [])).toEqual([
      7, 7, 7, 255, 8, 8, 8, 255, 9, 9, 9, 0,
      1, 1, 1, 255, 2, 2, 2, 0, 3, 3, 3, 255,
      4, 4, 4, 255, 5, 5, 5, 0, 6, 6, 6, 255,
    ]);
  });

  it('is not given while a row is missing', () => {
    expect(stripImage([block(0, 1, 2, [1, 2], [3]), block(2, 1, 2, [5, 6], [3])], 3, 2)).toBeNull();
    expect(stripImage([], 1, 2)).toBeNull();
  });

  it('is refused when a row comes twice', () => {
    expect(stripImage([block(0, 2, 2, [1, 2, 3, 4], [15]), block(1, 2, 2, [5, 6, 7, 8], [15])], 3, 2)).toBeNull();
  });

  it('is refused when a block does not fit the strip or hold its bytes', () => {
    expect(stripImage([block(0, 1, 3, [1, 2, 3], [7])], 1, 2)).toBeNull();
    expect(stripImage([block(0, 2, 2, [1, 2, 3, 4], [15])], 1, 2)).toBeNull();
    expect(stripImage([block(0, 1, 2, [1], [3])], 1, 2)).toBeNull();
    expect(stripImage([block(0, 1, 2, [1, 2], [])], 1, 2)).toBeNull();
  });

  it('covers every row exactly once, whatever blocks it was cut into and in whatever order they come', () => {
    const strips = fc
      .record({ rows: fc.integer({ min: 1, max: 40 }), columns: fc.integer({ min: 1, max: 9 }) })
      .chain(({ rows, columns }) =>
        fc.record({
          rows: fc.constant(rows),
          columns: fc.constant(columns),
          grey: fc.array(fc.integer({ min: 0, max: 255 }), { minLength: rows * columns, maxLength: rows * columns }),
          usable: fc.array(fc.boolean(), { minLength: rows * columns, maxLength: rows * columns }),
          cuts: fc.uniqueArray(fc.integer({ min: 1, max: Math.max(1, rows - 1) }), { maxLength: rows }),
          order: fc.array(fc.nat(), { minLength: rows, maxLength: rows }),
        }),
      );
    fc.assert(
      fc.property(strips, ({ rows, columns, grey, usable, cuts, order }) => {
        const edges = [0, ...cuts.filter((cut) => cut < rows).sort((a, b) => a - b), rows];
        const blocks = edges.slice(1).map((end, i) => {
          const start = edges[i] ?? 0;
          const cells = { from: start * columns, to: end * columns };
          return block(start, end - start, columns, grey.slice(cells.from, cells.to), packBits(usable.slice(cells.from, cells.to)));
        });
        const shuffled = blocks.map((b, i) => ({ b, key: order[i] ?? 0 })).sort((p, q) => p.key - q.key).map(({ b }) => b);

        const image = stripImage(shuffled, rows, columns);
        expect(image).not.toBeNull();
        for (let cell = 0; cell < rows * columns; cell++) {
          const level = grey[cell] ?? -1;
          expect(Array.from(image?.rgba.slice(cell * 4, cell * 4 + 4) ?? [])).toEqual([level, level, level, usable[cell] === true ? 255 : 0]);
        }
      }),
    );
  });
});
