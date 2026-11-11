// The levels a film's count counts: from T1 down to the count's last level,
// in the order the count gives them.

import { TEMPLATE_LEVELS, compareLevels } from '@/shapes';
import type { FilmMarks, Level } from '@/shapes';

/** The count's levels from T1 down, top to bottom. */
export function countedLevels(film: FilmMarks): readonly Level[] {
  return TEMPLATE_LEVELS[film.naming.template].filter((level) => compareLevels(level, 'T1') >= 0);
}
