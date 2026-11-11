// A film's record, and which opening of the film it is → a stamp.
//
// A strip, or a whole recording, was worked out from one film as it stood:
// its pixels, the quarter turns that show it upright, the brightness it
// was read with and the boxes blanked out of it. The stamp changes when
// any of those does, so a strip is given out only while its stamp still
// matches. Replacing a film, or opening it again, counts as a new opening.
// The facing, the scale and the confirmations do not change the strip and
// are not in the stamp.
//
// This area may not read the case, so the steps make the stamp and hand
// it in.

import type { Film } from '@/shapes';

/** Equal for two films exactly when a strip worked out from one is still the other's. */
export type FilmStamp = string;

/** `opening` counts the films opened into this film's slot this session. */
export function filmStamp(film: Film, opening: number): FilmStamp {
  const blanked = film.blanked.map((box) => [box.left, box.top, box.right, box.bottom]);
  return JSON.stringify([film.kind, opening, film.geometry.quarterTurns, film.values.inverted, blanked]);
}
