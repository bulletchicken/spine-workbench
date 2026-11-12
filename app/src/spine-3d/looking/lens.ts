// The lens: how a point in the scene lands on the canvas, from no
// perspective at all to the narrow lens the spine is turned in.
//
// With no perspective a point lands straight across from where it is, as
// on a film, so a bone sits on its outline whatever its depth. With the
// lens, the eye stands back along the view's axis at the distance from
// which the whole picture fills an 18 degree angle, and a point nearer than
// the aim is drawn that much larger. The aim itself never moves: it is
// drawn at the same place and scale at every perspective, which is what
// lets a view ease from flat to lens with nothing sliding.
//
// `toCanvas` is the rule in plain arithmetic. `lensMatrices` is the same
// rule as the two matrices a GPU wants; a test holds them to each other.

import type { Vec3 } from '../placing/vectors';
import { dot, sub } from '../placing/vectors';
import { type CanvasSize, type View, viewAxes } from './view';

/** The lens's full angle from the top of the picture to the bottom, degrees. */
export const LENS_DEGREES = 18;

/** Below this much perspective the view is drawn with none, exactly. */
export const FLAT_BELOW = 1e-4;

/** How far the eye is from the aim, mm; `null` when there is no perspective. */
export function eyeDistance(view: View, canvas: CanvasSize): number | null {
  if (view.perspective < FLAT_BELOW) return null;
  const halfHeight = canvas.height / (2 * view.pxPerMm[1]);
  return halfHeight / Math.tan((view.perspective * LENS_DEGREES * Math.PI) / 360);
}

export interface OnCanvas {
  /** CSS pixels from the canvas's left edge. */
  x: number;
  /** CSS pixels from its top edge. */
  y: number;
  /** Millimetres nearer the viewer than the aim (negative: further). */
  nearer: number;
}

/** Where a scene point is on the canvas. */
export function toCanvas(view: View, canvas: CanvasSize, point: Vec3): OnCanvas {
  const axes = viewAxes(view.azimuth, view.elevation);
  const from = sub(point, view.aim);
  const nearer = dot(from, axes.toViewer);
  const distance = eyeDistance(view, canvas);
  const grown = distance === null ? 1 : distance / (distance - nearer);
  return {
    x: view.aimAt[0] + view.pxPerMm[0] * dot(from, axes.right) * grown,
    y: view.aimAt[1] - view.pxPerMm[1] * dot(from, axes.up) * grown,
    nearer,
  };
}

export interface LensMatrices {
  /** The camera's place and directions in the scene: a 4 × 4 matrix, column by column. */
  world: number[];
  /** Camera space to clip space: a 4 × 4 matrix, column by column. */
  projection: number[];
  /** True when there is no perspective, and light should be worked out as for a parallel view. */
  flat: boolean;
}

/**
 * The view as a camera. `reach` is how far from the aim, in mm, anything
 * that must be drawn can be: nearer and further than that is cut away.
 */
export function lensMatrices(view: View, canvas: CanvasSize, reach: number): LensMatrices {
  const axes = viewAxes(view.azimuth, view.elevation);
  const distance = eyeDistance(view, canvas);
  // With no perspective the camera only has to stand outside what it draws.
  const back = distance ?? 2 * reach;
  const eye: Vec3 = [
    view.aim[0] + axes.toViewer[0] * back,
    view.aim[1] + axes.toViewer[1] * back,
    view.aim[2] + axes.toViewer[2] * back,
  ];
  const world = [...axes.right, 0, ...axes.up, 0, ...axes.toViewer, 0, ...eye, 1];

  // The aim, in the −1 to 1 square the GPU draws into.
  const aimX = (2 * view.aimAt[0]) / canvas.width - 1;
  const aimY = 1 - (2 * view.aimAt[1]) / canvas.height;
  const acrossScale = (2 * view.pxPerMm[0]) / canvas.width;
  const upScale = (2 * view.pxPerMm[1]) / canvas.height;
  const near = Math.max(back - reach, back * 0.02);
  const far = back + reach;

  if (distance === null) {
    // x' = aimX + acrossScale · x, and depth from −1 at `near` to 1 at `far`.
    const projection = [acrossScale, 0, 0, 0, 0, upScale, 0, 0, 0, 0, -2 / (far - near), 0, aimX, aimY, -(far + near) / (far - near), 1];
    return { world, projection, flat: true };
  }
  // x' = aimX + acrossScale · x · distance / (−z): the GPU divides by w = −z.
  const projection = [
    acrossScale * distance, 0, 0, 0,
    0, upScale * distance, 0, 0,
    -aimX, -aimY, -(far + near) / (far - near), -1,
    0, 0, (-2 * far * near) / (far - near), 0,
  ];
  return { world, projection, flat: false };
}
