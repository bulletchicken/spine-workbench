// Which vertebra is L5, when counting down from the first rib and counting
// up from the sacrum name different vertebrae L5. Upper is candidate A,
// lower is B.

import type { FilmMarks, VertebraOnFilm } from '@/shapes';

export interface L5Candidates {
  upper: VertebraOnFilm;
  lower: VertebraOnFilm;
}

/** True when the film has both counts and they name its vertebrae differently. */
export function countsDisagree(film: FilmMarks): boolean {
  const { fromTop, fromBottom } = film.naming;
  if (!fromTop || !fromBottom) return false;
  return fromTop.length !== fromBottom.length || fromTop.some((level, index) => level !== fromBottom[index]);
}

/** The two vertebrae each count calls L5, when the counts disagree and those are two different vertebrae. */
export function disagreeingL5(film: FilmMarks): L5Candidates | null {
  if (!countsDisagree(film)) return null;
  const top = film.naming.fromTop?.indexOf('L5') ?? -1;
  const bottom = film.naming.fromBottom?.indexOf('L5') ?? -1;
  if (top < 0 || bottom < 0 || top === bottom) return null;
  const upper = film.vertebrae[Math.min(top, bottom)];
  const lower = film.vertebrae[Math.max(top, bottom)];
  if (!upper || !lower) return null;
  return { upper, lower };
}

/** True while the question is open: the counts name different L5s and nobody has said which is right. */
export function l5QuestionOpen(film: FilmMarks): boolean {
  return film.naming.l5ConfirmedOn === null && disagreeingL5(film) !== null;
}
