// A bone pressed flat onto the film it was found on, and every step between
// that and where the fit stands it.
//
// Pressing moves a point straight along the film's beam toward the film's
// sheet. Seen along the beam with no perspective nothing moves at all, so a
// pressed bone still sits exactly on its outline; turned, it is a flat
// cut-out lying on the film. `amount` runs from 0 (on the sheet) to 1
// (where the fit put it).

import type { FilmPlane } from './film-plane';
import { type Mat3, type Vec3, dot, scale } from './vectors';

/** A stretch and a shift: `point → linear × point + offset`. */
export interface Affine {
  linear: Mat3;
  offset: Vec3;
}

/**
 * A bone is never pressed quite flat: a sheet with no thickness has no
 * front for light to fall on, and its bones would share one depth. This
 * much of the depth is kept (a twentieth of a millimetre in a spine 100 mm
 * deep), which keeps each bone in front of what it was in front of.
 */
export const THINNEST = 0.0005;

/** The move that leaves `amount` of each point's distance from the film's sheet. */
export function pressedOnto(plane: Pick<FilmPlane, 'towardReader' | 'out'>, amount: number): Affine {
  const kept = Math.min(1, Math.max(THINNEST, amount));
  const n = plane.towardReader;
  const lost = 1 - kept;
  // p − lost · ((p · n) − out) · n
  return {
    linear: [
      [1 - lost * n[0] * n[0], -lost * n[0] * n[1], -lost * n[0] * n[2]],
      [-lost * n[1] * n[0], 1 - lost * n[1] * n[1], -lost * n[1] * n[2]],
      [-lost * n[2] * n[0], -lost * n[2] * n[1], 1 - lost * n[2] * n[2]],
    ],
    offset: scale(n, lost * plane.out),
  };
}

/** A point after the move. */
export function moved(by: Affine, point: Vec3): Vec3 {
  return [
    dot(by.linear[0], point) + by.offset[0],
    dot(by.linear[1], point) + by.offset[1],
    dot(by.linear[2], point) + by.offset[2],
  ];
}
