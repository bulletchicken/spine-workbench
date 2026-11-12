// Stand a film from the fit's reported scale and origin, with only its
// facing, quarter turns and stored size supplied by the caller. A side
// film's scale already includes the fit's side-scale correction.

import type { FilmGeometry, FilmPlacement } from '@/shapes';
import { filmCamera } from './film-camera';

export function fittedFilmCamera(
  geometry: Pick<FilmGeometry, 'quarterTurns' | 'imageRight'>,
  size: readonly [number, number],
  placement: FilmPlacement,
) {
  return filmCamera(
    { ...geometry, mmPerPx: placement.mmPerPx[0], mmPerPxVertical: placement.mmPerPx[1] },
    size,
    placement.originPx,
  );
}
