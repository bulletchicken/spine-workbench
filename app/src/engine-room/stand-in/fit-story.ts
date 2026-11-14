import type { FitFromFilms, SpineFit } from '@/shapes';
import pair from '@/spine-3d/made-up-case/fitted-spine.json';
import front from '@/spine-3d/made-up-case/front-only-spine.json';
export function fitStory(input: FitFromFilms): SpineFit {
  const fit = structuredClone(input.side === null ? front.fit : pair.fit) as unknown as SpineFit;
  const levels = new Set(input.front.marks.vertebrae.map((v) => v.level));
  fit.vertebrae = fit.vertebrae.filter((v) => levels.has(v.level));
  fit.converged = fit.vertebrae.length > 1;
  fit.frontFilm.facingAssumed = input.front.facingAssumed;
  if (fit.sideFilm !== null) fit.sideFilm.facingAssumed = input.side?.facingAssumed ?? false;
  return fit;
}
