// The levels from T1 down to L5 that a film's count gives and that have no
// vertebra on the film. This is the engine's own definition of a missing
// vertebra, worked out from the case alone.

import { compareLevels } from '@/shapes';
import type { FilmMarks, Level } from '@/shapes';
import { countedLevels } from './counted-levels';

/** The missing levels, top to bottom. */
export function missingLevels(film: FilmMarks): Level[] {
  const present = new Set<Level>(film.vertebrae.map((vertebra) => vertebra.level));
  return countedLevels(film).filter((level) => compareLevels(level, 'L5') <= 0 && !present.has(level));
}
