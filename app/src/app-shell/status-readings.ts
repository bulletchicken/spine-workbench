// What the status line reads off the case and the pointer: where the
// pointer is on a film, in millimetres, and how big that film's pixels
// are. A length is never shown without its unit, and a film with no size
// is read in pixels and says so.

import { filmOf } from '@/case-state/selectors';
import type { CaseFileV1, FilmKind } from '@/shapes';
import type { PointerReading } from '@/view-state/pointer';

export interface Position {
  x: number;
  y: number;
  unit: 'mm' | 'px';
}

/**
 * Where the pointer is on its film, from the film's top-left corner: in
 * whole millimetres when the film has a size, in whole pixels when it has
 * none. `null` when the pointer is on no film, or on one the case does
 * not hold.
 */
export function positionOf(reading: PointerReading | null, c: CaseFileV1): Position | null {
  if (reading === null) return null;
  const film = filmOf(c, reading.film);
  if (film === null) return null;
  const [x, y] = reading.at;
  const { mmPerPx, mmPerPxVertical } = film.geometry;
  if (mmPerPx === null) return { x: Math.round(x), y: Math.round(y), unit: 'px' };
  return { x: Math.round(x * mmPerPx), y: Math.round(y * (mmPerPxVertical ?? mmPerPx)), unit: 'mm' };
}

/** A pixel size as it is written: two or three decimals, "0.45" and "0.144". */
export function pixelSizeWritten(mmPerPx: number): string {
  return mmPerPx.toFixed(3).replace(/0$/, '');
}

/**
 * The pixel size to show, as written, or `null` for none: that of the film
 * under the pointer; with the pointer on no film, the one size every sized
 * film shares, since films of one visit usually share one and a single
 * number then speaks for all of them.
 */
export function pixelSizeShown(c: CaseFileV1, under: FilmKind | null): string | null {
  if (under !== null) {
    const size = filmOf(c, under)?.geometry.mmPerPx ?? null;
    return size === null ? null : pixelSizeWritten(size);
  }
  const sizes = new Set(c.films.flatMap((film) => (film.geometry.mmPerPx === null ? [] : [pixelSizeWritten(film.geometry.mmPerPx)])));
  const [only] = sizes;
  return sizes.size === 1 && only !== undefined ? only : null;
}
