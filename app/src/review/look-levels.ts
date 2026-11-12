import { compareLevels, type FilmKind, type FilmMarks, type Level } from '@/shapes';
import { filledIn, needsALook } from './needs-a-look';

export interface LookLevel {
  level: Level;
  why: 'filled-in' | 'less-sure';
  on: Partial<Record<FilmKind, number>>;
}

/** One review per name, including that name's outlines on the other films. */
export function lookLevels(films: readonly FilmMarks[]): LookLevel[] {
  const levels = new Map<Level, LookLevel>();
  for (const film of films) for (const v of film.vertebrae) {
    if (compareLevels(v.level, 'T1') < 0 || !needsALook(v)) continue;
    const was = levels.get(v.level);
    levels.set(v.level, { level: v.level, why: filledIn(v) || was?.why === 'filled-in' ? 'filled-in' : 'less-sure', on: {} });
  }
  for (const film of films) for (const v of film.vertebrae) {
    const entry = levels.get(v.level);
    if (entry !== undefined) entry.on[film.film] = v.id;
  }
  return [...levels.values()].sort((a, b) => compareLevels(a.level, b.level));
}
