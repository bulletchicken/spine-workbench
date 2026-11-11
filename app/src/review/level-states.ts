// Each level on Front and Side, top to bottom: the vertebra each film has
// at it, how sure each film is, and the level's state across the two. The
// review queue walks these and the tally counts them.

import { compareLevels } from '@/shapes';
import type { FilmKind, FilmMarks, Level } from '@/shapes';
import type { Conf } from '@/look-and-feel/data-attributes';
import { howSure, levelState } from './how-sure';

export interface OnFilm {
  film: FilmKind;
  id: number;
  state: Conf;
}

export interface LevelOnFilms {
  level: Level;
  /** The most urgent state either film shows; `sacrum` when neither counts the level. */
  state: Conf;
  /** In the order the films were given. */
  onFilms: OnFilm[];
}

/** Every level with a vertebra on Front or Side, top to bottom. */
export function levelStates(films: readonly FilmMarks[]): LevelOnFilms[] {
  const frontAndSide = films.filter((film) => film.film === 'front' || film.film === 'side');
  const levels = [...new Set(frontAndSide.flatMap((film) => film.vertebrae.map((vertebra) => vertebra.level)))].sort(compareLevels);
  return levels.map((level) => {
    const onFilms = frontAndSide.flatMap((film) => {
      const vertebra = film.vertebrae.find((candidate) => candidate.level === level);
      return vertebra ? [{ film: film.film, id: vertebra.id, state: howSure(film, vertebra) }] : [];
    });
    return { level, state: levelState(onFilms.map((entry) => entry.state)), onFilms };
  });
}
