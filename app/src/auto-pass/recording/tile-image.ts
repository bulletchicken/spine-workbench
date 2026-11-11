// A preview tile → a bitmap and where it sits on the film.
//
// Each cell becomes one opaque grey pixel whose level is the cell's byte,
// as the tile's encoding wrote it (for a `signed-pair` tile, the first of
// the two bytes: the ridge value). Cell (i, j) is centred on the film at
// origin + i·stepAcross + j·stepDown, so the bitmap pixel square from
// (i, j) to (i + 1, j + 1) maps there through `toFilm`.

import type { PreviewTile } from '@/shapes';
import type { TileImage } from './pass-recording';

/** The bitmap of a tile, or null when its bytes do not fill its cells exactly. */
export function tileImage(tile: PreviewTile): TileImage | null {
  const cells = tile.width * tile.height;
  const bytesPerCell = tile.encoding === 'signed-pair' ? 2 : 1;
  if (tile.data.length !== cells * bytesPerCell) return null;

  const rgba = new Uint8ClampedArray(cells * 4);
  for (let cell = 0; cell < cells; cell++) {
    const level = tile.data[cell * bytesPerCell] ?? 0;
    rgba[cell * 4] = level;
    rgba[cell * 4 + 1] = level;
    rgba[cell * 4 + 2] = level;
    rgba[cell * 4 + 3] = 255;
  }

  const [ax, ay] = tile.stepAcross;
  const [dx, dy] = tile.stepDown;
  const [ox, oy] = tile.origin;
  return {
    width: tile.width,
    height: tile.height,
    rgba,
    // The pixel centre (i + ½, j + ½) must land on origin + i·across + j·down.
    toFilm: [ax, ay, dx, dy, ox - (ax + dx) / 2, oy - (ay + dy) / 2],
  };
}
