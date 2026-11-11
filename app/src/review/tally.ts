// How many counted levels are in each of the four states. A level that is
// not counted is left out of every number.

import type { FilmMarks } from '@/shapes';
import type { Conf } from '@/look-and-feel/data-attributes';
import { levelStates } from './level-states';

export interface Tally {
  sure: number;
  look: number;
  needs: number;
  byyou: number;
}

/** The count of each state over the given level states. */
export function countStates(states: readonly Conf[]): Tally {
  const counts: Tally = { sure: 0, look: 0, needs: 0, byyou: 0 };
  for (const state of states) {
    if (state !== 'sacrum') counts[state] += 1;
  }
  return counts;
}

/** The count of each state over the levels Front and Side count. */
export function tally(films: readonly FilmMarks[]): Tally {
  return countStates(levelStates(films).map((level) => level.state));
}
