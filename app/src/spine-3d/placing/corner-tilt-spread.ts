// One standard deviation of the tilt read from four independent corners.
// Each side's midpoint halves the variance; their difference restores it.
// A shared shift of all four corners cancels. This is a first-order band
// from the input corner spread, separate from the model angle's fit band.

import { type FilmCamera, type Px, filmToUpright } from './film-camera';

export function cornerTiltSpread(camera: FilmCamera, corners: Record<'supL' | 'supR' | 'infL' | 'infR', Px>, sdMm: number): number {
  const supL = filmToUpright(camera, corners.supL);
  const infL = filmToUpright(camera, corners.infL);
  const supR = filmToUpright(camera, corners.supR);
  const infR = filmToUpright(camera, corners.infR);
  const [su, sv] = camera.uprightMmPerPx;
  const across = ((supL[0] + infL[0] - supR[0] - infR[0]) / 2) * su;
  const up = ((supL[1] + infL[1] - supR[1] - infR[1]) / 2) * sv;
  return sdMm / Math.hypot(across, up);
}
