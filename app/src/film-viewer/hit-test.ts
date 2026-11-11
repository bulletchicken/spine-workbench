// What a point on a film is on or near, as pure geometry in film pixels.
// Reach is judged in screen pixels, because a target must be as easy to
// hit at fit as at six times the fit.

import type { Px } from '@/shapes';
import type { FilmSpace } from './film-space';

/** How near a pin answers, in screen pixels, whatever its dot looks like. */
export const PIN_REACH = 12;

/**
 * True when `p` is inside the polygon `corners` (an outline's four
 * corners, in order), by the even-odd rule. A point exactly on a top or
 * left edge is inside and one on a bottom or right edge is outside, so
 * two outlines that share an edge never both claim a point on it.
 */
export function pointInQuad(p: Px, corners: readonly Px[]): boolean {
  let inside = false;
  for (let i = 0, j = corners.length - 1; i < corners.length; j = i, i += 1) {
    const a = corners[i];
    const b = corners[j];
    if (a === undefined || b === undefined) continue;
    if (a[1] > p[1] === b[1] > p[1]) continue;
    const crossing = a[0] + ((p[1] - a[1]) * (b[0] - a[0])) / (b[1] - a[1]);
    if (p[0] < crossing) inside = !inside;
  }
  return inside;
}

/**
 * The index of the point nearest `at` that lies within `reach` screen
 * pixels of it, or `null`. Distances are measured on screen, through the
 * film's current placement.
 */
export function nearestWithin(points: readonly Px[], at: Px, reach: number, space: FilmSpace): number | null {
  const pointer = space.toScreen(at);
  let best: number | null = null;
  let bestDistance = reach;
  points.forEach((point, i) => {
    const [x, y] = space.toScreen(point);
    const distance = Math.hypot(x - pointer[0], y - pointer[1]);
    if (distance <= bestDistance) {
      best = i;
      bestDistance = distance;
    }
  });
  return best;
}
