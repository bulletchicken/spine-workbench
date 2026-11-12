import { FILM_KINDS } from '@/case-state/selectors';
import type { EngineClient } from '@/engine-room/engine-client';
import type { FilmPng, MadeUpRecipe } from '@/shapes';

// The first seed with T1 to L5 on both standing films; earlier seeds also named L6.
export const SAMPLE_RECIPE: MadeUpRecipe = { name: 'right-thoracic-40', seed: 4, tier: 't1' };

export async function makeTheSampleCase(engine: Pick<EngineClient, 'makeUpSpine'>): Promise<FilmPng[]> {
  const { films } = await engine.makeUpSpine(SAMPLE_RECIPE);
  return FILM_KINDS.flatMap((kind) => films.filter((film) => film.kind === kind));
}
