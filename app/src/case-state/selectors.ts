// Reading the case: each film's record and marks, one vertebra, and the
// films in their fixed order. Pure functions over a `CaseFileV1`, used by
// the reducer and by the screens alike.

import type { CaseFileV1, Film, FilmKind, FilmMarks, VertebraOnFilm } from '@/shapes';

const ORDER: Record<FilmKind, number> = { front: 0, side: 1, 'bend-left': 2, 'bend-right': 3 };

/** Every film kind, in the order the case keeps its films and marks. */
export const FILM_KINDS = Object.keys(ORDER) as readonly FilmKind[];

/** Negative when film kind `a` comes before `b` in the case. */
export function compareFilmKinds(a: FilmKind, b: FilmKind): number {
  return ORDER[a] - ORDER[b];
}

/** The film of this kind, or `null` when the case has none. */
export function filmOf(c: CaseFileV1, kind: FilmKind): Film | null {
  return c.films.find((film) => film.kind === kind) ?? null;
}

/** That film's marks, or `null` until it has any. */
export function marksOf(c: CaseFileV1, kind: FilmKind): FilmMarks | null {
  return c.marks.find((marks) => marks.film === kind) ?? null;
}

/** One vertebra on one film, by id, or `null` when that film does not hold it. */
export function vertebraOf(c: CaseFileV1, kind: FilmKind, id: number): VertebraOnFilm | null {
  return marksOf(c, kind)?.vertebrae.find((vertebra) => vertebra.id === id) ?? null;
}

/** One above the highest vertebra id in the case, or 0 when it has none. */
export function aboveEveryId(c: CaseFileV1): number {
  let above = 0;
  for (const marks of c.marks) for (const vertebra of marks.vertebrae) above = Math.max(above, vertebra.id + 1);
  return above;
}
