// The two ends of an endplate's rim as a film shows them: the corners of
// the vertebral body on that film. The engine finds them in closed form
// (engine/forward-model/src/corners.rs); this is the same rule, worked on a
// rim given as three points, so it can be asked of a shaped bone's own rim.
//
// A rim is an oval: `centre + toLeft · cos t + toBack · sin t`. On a film
// it lands as an oval too, and the body's corner there is the end of that
// oval along the rim's long direction in the view: across it on a front or
// bending film, front to back on a side film.

import { type FilmCamera, screenRight } from './film-camera';
import { type Vec3, add, dot, scale, sub } from './vectors';

/** An endplate's rim: its centre, and the offsets from there to its left end and to its back end. */
export interface Rim {
  centre: Vec3;
  toLeft: Vec3;
  toBack: Vec3;
}

/** A rim from three of its landmarks. */
export function rimThrough(centre: Vec3, left: Vec3, back: Vec3): Rim {
  return { centre, toLeft: sub(left, centre), toBack: sub(back, centre) };
}

/** A direction in the patient as it lies on the upright film, in pixels per millimetre. */
function onFilm(camera: FilmCamera, direction: Vec3): [number, number] {
  const [su, sv] = camera.uprightMmPerPx;
  return [dot(direction, screenRight(camera.imageRight)) / su, -direction[2] / sv];
}

/**
 * The rim's two ends on `camera`'s film, as points in space: first the
 * patient's left one (`across`) or the front one (`front-to-back`), then
 * the other.
 */
export function rimEnds(rim: Rim, camera: FilmCamera, axis: 'across' | 'front-to-back'): [Vec3, Vec3] {
  const a = onFilm(camera, rim.toLeft);
  const b = onFilm(camera, rim.toBack);
  const long = axis === 'across' ? a : ([-b[0], -b[1]] as const);
  const alongA = long[0] * a[0] + long[1] * a[1];
  const alongB = long[0] * b[0] + long[1] * b[1];
  const reach = Math.hypot(alongA, alongB);
  const offset = add(scale(rim.toLeft, alongA / reach), scale(rim.toBack, alongB / reach));
  return [add(rim.centre, offset), sub(rim.centre, offset)];
}
