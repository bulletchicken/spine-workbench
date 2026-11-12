// One made-up spine, fitted by the engine from the true marks of its front
// and side films: what the 3D view's tests check their placing against and
// what its gallery pages draw. Nothing in it comes from a patient. See the
// README beside this file for how it was made.

import type { FilmGeometry, Level, MarkSpread, SpineFit } from '@/shapes';
import type { Px } from '../placing/film-camera';
import written from './fitted-spine.json';
import frontOnly from './front-only-spine.json';

/** One vertebra's corners on a film: where they were marked, and where the fit's own vertebra lands. */
export interface CornersOnFilm<Name extends string> {
  level: Level;
  marked: Record<Name, Px>;
  fitted: Record<Name, Px>;
}

export interface MadeUpCase {
  what: string;
  /** The recipe, seed and tier the spine was made up from. */
  case: string;
  fit: SpineFit;
  front: { markSpread: MarkSpread; geometry: FilmGeometry; size: [number, number]; vertebrae: CornersOnFilm<'supL' | 'supR' | 'infL' | 'infR'>[] };
  side: { geometry: FilmGeometry; size: [number, number]; vertebrae: CornersOnFilm<'supA' | 'supP' | 'infA' | 'infP'>[] } | null;
}

// The file is written by the engine's own shapes, so it is taken as they are typed.
export const MADE_UP_CASE = written as unknown as MadeUpCase & { side: NonNullable<MadeUpCase['side']> };

/** The same case, fitted from front corners alone, with no stated scale. */
export const FRONT_ONLY_CASE = frontOnly as unknown as MadeUpCase;
