// The vertebra on one film that is the same level as a vertebra on another,
// found by level name. Ids differ between films, so this is how one vertebra
// is followed across them. When the two films do not count alike (see
// same-count.ts), no level is linked at all, so a link goes missing rather
// than land on another bone.

import type { FilmKind, FilmMarks } from '@/shapes';
import { countAlike } from './same-count';

/** A vertebra's id on each film it is linked on. */
export type FilmIds = Partial<Record<FilmKind, number>>;

/** The id of the vertebra on `to` with the same level as vertebra `vertebraId` on `from`, or null. */
export function sameLevelOn(from: FilmMarks, to: FilmMarks, vertebraId: number): number | null {
  if (!countAlike(from, to)) return null;
  const vertebra = from.vertebrae.find((candidate) => candidate.id === vertebraId);
  if (!vertebra) return null;
  return to.vertebrae.find((candidate) => candidate.level === vertebra.level)?.id ?? null;
}

/** Vertebra `vertebraId` of `from`, followed onto every film it links to. */
export function idsAcross(films: readonly FilmMarks[], from: FilmMarks, vertebraId: number): FilmIds {
  const ids: FilmIds = { [from.film]: vertebraId };
  for (const film of films) {
    if (film.film === from.film) continue;
    const id = sameLevelOn(from, film, vertebraId);
    if (id !== null) ids[film.film] = id;
  }
  return ids;
}
