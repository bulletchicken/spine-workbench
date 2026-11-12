// What the outlines on a film say to a screen reader: the group of one
// film's outlines, and each outline's name.

import type { FilmKind } from '@/shapes';
import { FILM_NAMES } from './film-names';

export const MARK_WORDS = {
  /** Every outline on one film, as a group: "Outlines on the Front film". */
  outlinesOn: (kind: FilmKind) => `Outlines on the ${FILM_NAMES[kind]} film`,
  /** One outline: "T3", or "T3, needs a look". */
  outline: (name: string, look: boolean) => (look ? `${name}, needs a look` : name),
} as const;
