// The deepest gap of a film's bone-and-gap profile between two film
// points, as a film point.
//
// The profile with slow trends removed reads above zero on bone and below
// zero on a disc gap. Between the strip rows of the two points, keeping a
// quarter of the span clear of each end, the gap is the lowest row that
// is below zero and no higher than either neighbour. It is given back as
// that row's centre on the film. With no such row there is no gap.

import type { Px } from '@/shapes';
import type { FilmStrip } from './pass-recording';
import { nearestRow } from './strip-position';

export function gapInSpan(strip: Pick<FilmStrip, 'shape' | 'barcode'>, from: Px, to: Px): Px | null {
  const a = nearestRow(strip.shape, from);
  const b = nearestRow(strip.shape, to);
  if (a === null || b === null) return null;
  const span = Math.abs(b - a);
  const first = Math.ceil(Math.min(a, b) + span / 4);
  const last = Math.floor(Math.max(a, b) - span / 4);

  const profile = strip.barcode.highPass;
  let deepest: number | null = null;
  for (let row = first; row <= last; row++) {
    const here = profile[row];
    if (here === undefined || !(here < 0)) continue;
    const above = profile[row - 1];
    const below = profile[row + 1];
    if ((above !== undefined && above < here) || (below !== undefined && below < here)) continue;
    if (deepest === null || here < (profile[deepest] ?? 0)) deepest = row;
  }
  return deepest === null ? null : (strip.shape.centres[deepest] ?? null);
}
