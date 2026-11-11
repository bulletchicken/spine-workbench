// strip-rows blocks → one bitmap of the unrolled strip.
//
// Each block carries one byte per cell and one validity bit per cell,
// row-major, lowest bit first. A usable cell becomes an opaque grey pixel
// of its byte; a cell that was not usable becomes a fully transparent
// pixel, so nothing is drawn where the engine had no film to sample.

import type { StripBlock, StripImage } from './pass-recording';

/**
 * The strip, once the blocks cover every one of its `rows` exactly once
 * at `columns` columns; null while a row is missing, or if any row came
 * twice or any block does not hold the bytes its size says.
 */
export function stripImage(blocks: readonly StripBlock[], rows: number, columns: number): StripImage | null {
  const covered = new Uint8Array(rows);
  for (const block of blocks) {
    const cells = block.rows * block.columns;
    if (block.columns !== columns || block.image.length !== cells || block.valid.length !== Math.ceil(cells / 8)) return null;
    if (block.firstRow < 0 || block.firstRow + block.rows > rows) return null;
    for (let row = block.firstRow; row < block.firstRow + block.rows; row++) {
      if (covered[row] === 1) return null;
      covered[row] = 1;
    }
  }
  if (covered.includes(0)) return null;

  const rgba = new Uint8ClampedArray(rows * columns * 4);
  for (const block of blocks) {
    const start = block.firstRow * columns;
    for (let cell = 0; cell < block.rows * block.columns; cell++) {
      const usable = (((block.valid[cell >> 3] ?? 0) >> (cell & 7)) & 1) === 1;
      const level = block.image[cell] ?? 0;
      const at = (start + cell) * 4;
      rgba[at] = level;
      rgba[at + 1] = level;
      rgba[at + 2] = level;
      rgba[at + 3] = usable ? 255 : 0;
    }
  }
  return { rows, columns, rgba };
}
