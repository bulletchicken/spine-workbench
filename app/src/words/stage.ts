import type { FilmKind } from '@/shapes';
import { FILM_NAMES } from './film-names';

export const STAGE_WORDS = {
  display: 'Turn four X-rays into a spine you can measure.',
  film: (kind: FilmKind) => `${FILM_NAMES[kind]} film`,
  choose: (kind: FilmKind) => `Choose the ${FILM_NAMES[kind]} film`,
  replace: (kind: FilmKind) => `Replace the ${FILM_NAMES[kind]} film`,
  reading: (kind: FilmKind) => `Reading the ${FILM_NAMES[kind]} film`,
};
